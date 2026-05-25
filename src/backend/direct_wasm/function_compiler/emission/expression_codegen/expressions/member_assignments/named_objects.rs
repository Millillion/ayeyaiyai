use super::*;

fn static_array_property_is_known_non_index(property: &Expression) -> bool {
    match property {
        Expression::String(text) => {
            text != "length" && argument_index_from_expression(property).is_none()
        }
        Expression::Number(_) => argument_index_from_expression(property).is_none(),
        _ => false,
    }
}

fn array_binding_static_expression(binding: &ArrayValueBinding) -> Expression {
    Expression::Array(
        binding
            .values
            .iter()
            .map(|value| ArrayElement::Expression(value.clone().unwrap_or(Expression::Undefined)))
            .collect(),
    )
}

fn write_array_binding_static_slot(
    binding: &mut ArrayValueBinding,
    index: u32,
    value: &Expression,
) {
    while binding.values.len() <= index as usize {
        binding.values.push(None);
    }
    binding.values[index as usize] = Some(value.clone());
}

fn member_assignment_expression_references_internal_iterator_step(expression: &Expression) -> bool {
    match expression {
        Expression::Identifier(name) => {
            name.starts_with("__ayy_array_step_") || name.starts_with("__ayy_for_of_step_")
        }
        Expression::Array(elements) => elements.iter().any(|element| match element {
            ArrayElement::Expression(value) | ArrayElement::Spread(value) => {
                member_assignment_expression_references_internal_iterator_step(value)
            }
        }),
        Expression::Object(entries) => entries.iter().any(|entry| match entry {
            ObjectEntry::Data { key, value } => {
                member_assignment_expression_references_internal_iterator_step(key)
                    || member_assignment_expression_references_internal_iterator_step(value)
            }
            ObjectEntry::Getter { key, getter } => {
                member_assignment_expression_references_internal_iterator_step(key)
                    || member_assignment_expression_references_internal_iterator_step(getter)
            }
            ObjectEntry::Setter { key, setter } => {
                member_assignment_expression_references_internal_iterator_step(key)
                    || member_assignment_expression_references_internal_iterator_step(setter)
            }
            ObjectEntry::Spread(value) => {
                member_assignment_expression_references_internal_iterator_step(value)
            }
        }),
        Expression::Binary { left, right, .. } => {
            member_assignment_expression_references_internal_iterator_step(left)
                || member_assignment_expression_references_internal_iterator_step(right)
        }
        Expression::Conditional {
            condition,
            then_expression,
            else_expression,
        } => {
            member_assignment_expression_references_internal_iterator_step(condition)
                || member_assignment_expression_references_internal_iterator_step(then_expression)
                || member_assignment_expression_references_internal_iterator_step(else_expression)
        }
        Expression::Member { object, property } => {
            member_assignment_expression_references_internal_iterator_step(object)
                || member_assignment_expression_references_internal_iterator_step(property)
        }
        Expression::Unary { expression, .. }
        | Expression::Await(expression)
        | Expression::EnumerateKeys(expression)
        | Expression::GetIterator(expression)
        | Expression::IteratorClose(expression) => {
            member_assignment_expression_references_internal_iterator_step(expression)
        }
        Expression::Assign { value, .. } => {
            member_assignment_expression_references_internal_iterator_step(value)
        }
        Expression::AssignMember {
            object,
            property,
            value,
        } => {
            member_assignment_expression_references_internal_iterator_step(object)
                || member_assignment_expression_references_internal_iterator_step(property)
                || member_assignment_expression_references_internal_iterator_step(value)
        }
        Expression::AssignSuperMember { property, value } => {
            member_assignment_expression_references_internal_iterator_step(property)
                || member_assignment_expression_references_internal_iterator_step(value)
        }
        Expression::Call { callee, arguments }
        | Expression::New { callee, arguments }
        | Expression::SuperCall { callee, arguments } => {
            member_assignment_expression_references_internal_iterator_step(callee)
                || arguments.iter().any(|argument| match argument {
                    CallArgument::Expression(value) | CallArgument::Spread(value) => {
                        member_assignment_expression_references_internal_iterator_step(value)
                    }
                })
        }
        Expression::SuperMember { property } => {
            member_assignment_expression_references_internal_iterator_step(property)
        }
        _ => false,
    }
}

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn test262_realm_constructor_member(
        &self,
        expression: &Expression,
    ) -> Option<(u32, String)> {
        let Expression::Member { object, property } = expression else {
            return None;
        };
        let Expression::String(constructor_name) = property.as_ref() else {
            return None;
        };
        if !matches!(
            constructor_name.as_str(),
            "Object"
                | "Function"
                | "Array"
                | "Number"
                | "String"
                | "Boolean"
                | "Symbol"
                | "BigInt"
                | "RegExp"
                | "Date"
                | "Error"
                | "TypeError"
                | "SyntaxError"
                | "RangeError"
                | "ReferenceError"
                | "AggregateError"
                | "EvalError"
                | "URIError"
        ) {
            return None;
        }
        let realm_id = self.resolve_test262_realm_global_id_from_expression(object)?;
        Some((realm_id, constructor_name.clone()))
    }

    pub(in crate::backend::direct_wasm) fn test262_realm_constructor_object_binding(
        &self,
        realm_id: u32,
        constructor_name: &str,
    ) -> ObjectValueBinding {
        let constructor_property = Expression::String(constructor_name.to_string());
        self.test262_realm_global_object_binding(realm_id)
            .and_then(|global_binding| {
                object_binding_lookup_value(&global_binding, &constructor_property).cloned()
            })
            .and_then(|value| self.resolve_object_binding_from_expression(&value))
            .unwrap_or_else(empty_object_value_binding)
    }

    fn named_member_assignment_property_requires_runtime_coercion(
        &self,
        property: &Expression,
    ) -> bool {
        if static_property_name_from_expression(property).is_some()
            || self.well_known_symbol_name(property).is_some()
        {
            return false;
        }

        match property {
            Expression::Identifier(name) => {
                let resolved_name = self
                    .resolve_current_local_binding(name)
                    .map(|(resolved_name, _)| resolved_name)
                    .unwrap_or_else(|| name.clone());
                self.state
                    .speculation
                    .static_semantics
                    .local_object_binding(&resolved_name)
                    .is_some()
                    || self
                        .state
                        .speculation
                        .static_semantics
                        .local_object_binding(name)
                        .is_some()
                    || self
                        .state
                        .speculation
                        .static_semantics
                        .local_kind(&resolved_name)
                        == Some(StaticValueKind::Object)
                    || self.state.speculation.static_semantics.local_kind(name)
                        == Some(StaticValueKind::Object)
                    || self.global_object_binding(name).is_some()
                    || self.global_binding_kind(name) == Some(StaticValueKind::Object)
            }
            Expression::Array(_) | Expression::Object(_) => true,
            _ => false,
        }
    }

    fn assignment_value_reads_same_member(
        value: &Expression,
        object: &Expression,
        property: &Expression,
    ) -> bool {
        let Expression::Binary { left, .. } = value else {
            return false;
        };
        matches!(
            left.as_ref(),
            Expression::Member {
                object: left_object,
                property: left_property,
            } if static_expression_matches(left_object, object)
                && static_expression_matches(left_property, property)
        )
    }

    fn invalidate_dynamic_named_member_assignment_target(&mut self, name: &str) {
        self.state.clear_local_static_binding_metadata(name);
        self.state.clear_member_bindings_for_name(name, true);
        if self.binding_name_is_global(name) {
            self.clear_global_binding_state(name);
            self.backend.clear_global_member_bindings_for_name(name);
        }
    }

    fn member_function_assignment_capture_source_expression(
        &self,
        capture_name: &str,
    ) -> Option<(Expression, bool)> {
        if capture_name == "new.target" {
            return Some((Expression::NewTarget, true));
        }
        if capture_name == "this" {
            if let Some(hidden_name) = self.resolve_user_function_capture_hidden_name("this") {
                return Some((Expression::Identifier(hidden_name), true));
            }
            if self.current_function_name().is_some() {
                return Some((Expression::This, true));
            }
            return self
                .global_has_binding("this")
                .then(|| (Expression::Identifier("this".to_string()), false));
        }

        if let Some(hidden_name) = self.resolve_user_function_capture_hidden_name(capture_name) {
            return Some((Expression::Identifier(hidden_name), true));
        }
        if let Some(scope_object) = self.resolve_with_scope_binding_for_specialization(capture_name)
        {
            return Some((
                Expression::Member {
                    object: Box::new(scope_object),
                    property: Box::new(Expression::String(capture_name.to_string())),
                },
                true,
            ));
        }
        if self.resolve_current_local_binding(capture_name).is_some() {
            return Some((Expression::Identifier(capture_name.to_string()), true));
        }
        if let Some(hidden_name) = self.resolve_eval_local_function_hidden_name(capture_name) {
            return Some((Expression::Identifier(hidden_name), true));
        }
        if self.global_has_binding(capture_name)
            || self.backend.global_has_lexical_binding(capture_name)
            || self.backend.global_function_binding(capture_name).is_some()
            || self.global_has_implicit_binding(capture_name)
        {
            return Some((Expression::Identifier(capture_name.to_string()), false));
        }
        None
    }

    fn member_function_assignment_existing_capture_slot_is_unboxed_runtime_binding(
        &self,
        capture_name: &str,
        slot_name: &str,
    ) -> bool {
        if self
            .state
            .speculation
            .static_semantics
            .capture_slot_source_bindings
            .contains_key(slot_name)
        {
            return false;
        }
        if self.state.runtime.locals.get(slot_name).is_none() {
            return false;
        }
        slot_name == capture_name
            || scoped_binding_source_name(slot_name).is_some_and(|source| source == capture_name)
            || self
                .resolve_current_local_binding(capture_name)
                .is_some_and(|(resolved_name, _)| resolved_name == slot_name)
    }

    pub(in crate::backend::direct_wasm) fn initialize_member_function_assignment_capture_slots(
        &mut self,
        object: &Expression,
        member_property: &Expression,
        value: &Expression,
    ) -> DirectResult<()> {
        let trace_capture_bindings = std::env::var_os("AYY_TRACE_CAPTURE_BINDINGS").is_some();
        let Some(LocalFunctionBinding::User(function_name)) =
            self.resolve_function_binding_from_expression(value)
        else {
            if trace_capture_bindings {
                eprintln!(
                    "capture_slots init skip non_user value={value:?} object={object:?} property={member_property:?}"
                );
            }
            return Ok(());
        };
        let Some(capture_bindings) = self
            .backend
            .function_registry
            .analysis
            .user_function_capture_bindings
            .get(&function_name)
            .filter(|captures| !captures.is_empty())
            .cloned()
        else {
            if trace_capture_bindings {
                eprintln!(
                    "capture_slots init skip no_captures function={function_name} value={value:?}"
                );
            }
            return Ok(());
        };
        let Some(key) = self.member_function_binding_key(object, member_property) else {
            if trace_capture_bindings {
                eprintln!(
                    "capture_slots init skip no_key function={function_name} object={object:?} property={member_property:?}"
                );
            }
            return Ok(());
        };
        if trace_capture_bindings {
            eprintln!(
                "capture_slots init start function={function_name} key={key:?} captures={capture_bindings:?} value={value:?}"
            );
        }
        let member_capture_is_runtime_array_element =
            argument_index_from_expression(member_property).is_some()
                && self
                    .runtime_array_binding_name_for_expression(object)
                    .is_some_and(|name| self.runtime_array_binding_has_state(&name));

        if let Some(existing_capture_slots) = self.resolve_function_expression_capture_slots(value)
        {
            let mut capture_slots = BTreeMap::new();
            for capture_name in capture_bindings.keys() {
                let Some(slot_name) = existing_capture_slots.get(capture_name) else {
                    capture_slots.clear();
                    break;
                };
                if self.member_function_assignment_existing_capture_slot_is_unboxed_runtime_binding(
                    capture_name,
                    slot_name,
                ) {
                    capture_slots.clear();
                    break;
                }
                capture_slots.insert(capture_name.clone(), slot_name.clone());
            }
            if !capture_slots.is_empty() {
                self.state
                    .speculation
                    .static_semantics
                    .objects
                    .member_function_capture_slots
                    .insert(key.clone(), capture_slots.clone());
                if trace_capture_bindings {
                    eprintln!(
                        "capture_slots init insert existing key={key:?} slots={capture_slots:?}"
                    );
                }
                if self.binding_key_is_global(&key) {
                    self.backend
                        .set_global_member_function_capture_slots(key, capture_slots);
                }
                return Ok(());
            }
        }

        let mut capture_slots = BTreeMap::new();
        for capture_name in capture_bindings.keys() {
            let Some((source_expression, source_is_runtime_local)) =
                self.member_function_assignment_capture_source_expression(capture_name)
            else {
                continue;
            };
            if source_is_runtime_local {
                let hidden_name = self.allocate_named_hidden_local(
                    &format!("member_closure_slot_{}_{}", function_name, capture_name),
                    self.infer_value_kind(&source_expression)
                        .unwrap_or(StaticValueKind::Unknown),
                );
                let hidden_local = self
                    .state
                    .runtime
                    .locals
                    .bindings
                    .get(&hidden_name)
                    .copied()
                    .expect("fresh member closure capture slot local must exist");
                self.emit_numeric_expression(&source_expression)?;
                self.push_local_set(hidden_local);
                self.update_capture_slot_binding_from_expression(&hidden_name, &source_expression)?;
                self.sync_capture_slot_runtime_object_shadows_from_expression(
                    &hidden_name,
                    &source_expression,
                )?;
                if let Expression::Identifier(source_binding_name) = &source_expression {
                    if member_capture_is_runtime_array_element {
                        capture_slots.insert(capture_name.clone(), hidden_name);
                        continue;
                    }
                    self.state
                        .speculation
                        .static_semantics
                        .capture_slot_source_bindings
                        .insert(
                            hidden_name.clone(),
                            self.capture_slot_live_source_binding_name(source_binding_name),
                        );
                } else if matches!(source_expression, Expression::This) {
                    self.state
                        .speculation
                        .static_semantics
                        .capture_slot_source_bindings
                        .insert(hidden_name.clone(), capture_name.clone());
                } else if matches!(source_expression, Expression::NewTarget) {
                    self.state
                        .speculation
                        .static_semantics
                        .capture_slot_source_bindings
                        .insert(hidden_name.clone(), capture_name.clone());
                } else if let Expression::Member { object, property } = &source_expression
                    && let Some(source_key) = Self::capture_slot_member_source_key(object, property)
                {
                    self.state
                        .speculation
                        .static_semantics
                        .capture_slot_source_bindings
                        .insert(hidden_name.clone(), source_key);
                }
                capture_slots.insert(capture_name.clone(), hidden_name);
            } else if let Expression::Identifier(source_binding_name) = source_expression {
                capture_slots.insert(capture_name.clone(), source_binding_name);
            }
        }

        if capture_slots.is_empty() {
            if trace_capture_bindings {
                eprintln!("capture_slots init skip empty key={key:?}");
            }
            return Ok(());
        }

        self.state
            .speculation
            .static_semantics
            .objects
            .member_function_capture_slots
            .insert(key.clone(), capture_slots.clone());
        if trace_capture_bindings {
            eprintln!("capture_slots init insert key={key:?} slots={capture_slots:?}");
        }
        if self.binding_key_is_global(&key) {
            self.backend
                .set_global_member_function_capture_slots(key, capture_slots);
        }

        Ok(())
    }

    fn push_dynamic_for_in_key_property_candidates(
        &self,
        expression: &Expression,
        candidates: &mut Vec<String>,
        depth: usize,
    ) {
        if depth > 8 {
            return;
        }
        if let Expression::Identifier(name) = expression {
            if let Some(value) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
                .or_else(|| self.global_value_binding(name))
                && !static_expression_matches(value, expression)
            {
                self.push_dynamic_for_in_key_property_candidates(value, candidates, depth + 1);
            }
        }

        let materialized = self.materialize_static_expression(expression);
        if !static_expression_matches(&materialized, expression) {
            self.push_dynamic_for_in_key_property_candidates(&materialized, candidates, depth + 1);
        }

        let Expression::Member { object, property } = expression else {
            return;
        };
        let Expression::Identifier(object_name) = object.as_ref() else {
            return;
        };
        if !object_name.starts_with("__ayy_for_in_keys_") {
            return;
        }
        let Some(array_binding) = self.resolve_array_binding_from_expression(object) else {
            return;
        };
        let materialized_property = self
            .resolve_property_key_expression(property)
            .unwrap_or_else(|| self.materialize_static_expression(property));
        if let Some(index) = argument_index_from_expression(&materialized_property) {
            if let Some(Some(Expression::String(property_name))) =
                array_binding.values.get(index as usize)
                && !candidates
                    .iter()
                    .any(|candidate| candidate == property_name)
            {
                candidates.push(property_name.clone());
            }
            return;
        }
        if !self.expression_depends_on_active_loop_assignment(property) {
            return;
        }
        for value in array_binding.values.iter().flatten() {
            let Expression::String(property_name) = value else {
                continue;
            };
            if !candidates
                .iter()
                .any(|candidate| candidate == property_name)
            {
                candidates.push(property_name.clone());
            }
        }
    }

    fn dynamic_for_in_key_property_candidates(&self, expression: &Expression) -> Vec<String> {
        let mut candidates = Vec::new();
        self.push_dynamic_for_in_key_property_candidates(expression, &mut candidates, 0);
        candidates
    }

    fn member_assignment_bigint_update_value(&self, value: &Expression) -> Option<Expression> {
        let Expression::Binary { op, left, right } = value else {
            return None;
        };
        if !matches!(op, BinaryOp::Add | BinaryOp::Subtract) {
            return None;
        }
        if !matches!(right.as_ref(), Expression::Number(number) if *number == 1.0) {
            return None;
        }
        let left_value = self
            .resolve_postfix_previous_bigint_value(left)
            .or_else(|| self.resolve_static_bigint_value(left))?;
        let unit = StaticBigInt::from(1);
        let updated = match op {
            BinaryOp::Add => left_value + unit,
            BinaryOp::Subtract => left_value - unit,
            _ => unreachable!("filtered above"),
        };
        Some(Expression::BigInt(updated.to_string()))
    }

    fn resolve_postfix_previous_bigint_value(
        &self,
        expression: &Expression,
    ) -> Option<StaticBigInt> {
        let Expression::Identifier(name) = expression else {
            return None;
        };
        if !name.starts_with("__ayy_postfix_previous_") {
            return None;
        }
        let value = self
            .resolve_current_local_binding(name)
            .and_then(|(resolved_name, _)| {
                self.state
                    .speculation
                    .static_semantics
                    .local_value_binding(&resolved_name)
            })
            .or_else(|| {
                self.state
                    .speculation
                    .static_semantics
                    .local_value_binding(name)
            })
            .or_else(|| self.global_value_binding(name))?;
        self.resolve_postfix_previous_bigint_candidate(value.clone())
    }

    fn resolve_postfix_previous_bigint_candidate(&self, value: Expression) -> Option<StaticBigInt> {
        let mut current = value;
        for _ in 0..6 {
            if let Some(value) = Self::literal_bigint_value(&current) {
                return Some(value);
            }
            if let Some(value) = self.member_expression_static_bigint_value(&current) {
                return Some(value);
            }
            let materialized = self.materialize_static_expression(&current);
            if static_expression_matches(&materialized, &current) {
                break;
            }
            current = materialized;
        }
        Self::literal_bigint_value(&current)
            .or_else(|| self.member_expression_static_bigint_value(&current))
    }

    fn literal_bigint_value(expression: &Expression) -> Option<StaticBigInt> {
        match expression {
            Expression::BigInt(value) => parse_static_bigint_literal(value),
            Expression::Unary {
                op: UnaryOp::Negate,
                expression,
            } => Some(-Self::literal_bigint_value(expression)?),
            _ => None,
        }
    }

    fn member_expression_static_bigint_value(
        &self,
        expression: &Expression,
    ) -> Option<StaticBigInt> {
        let Expression::Member { object, property } = expression else {
            return None;
        };

        let mut object_candidates = vec![object.as_ref().clone()];
        if let Some(resolved) = self.resolve_bound_alias_expression(object) {
            object_candidates.push(resolved);
        }
        object_candidates.push(self.materialize_static_expression(object));

        let mut property_candidates = vec![property.as_ref().clone()];
        if let Some(resolved) = self.resolve_property_key_expression(property) {
            property_candidates.push(resolved);
        }
        if let Some(resolved) = self.resolve_bound_alias_expression(property) {
            property_candidates.push(resolved);
        }
        property_candidates.push(self.materialize_static_expression(property));

        for object_candidate in object_candidates {
            for property_candidate in &property_candidates {
                if let Some(value) = self
                    .resolve_object_binding_from_expression(&object_candidate)
                    .and_then(|binding| {
                        object_binding_lookup_value(&binding, property_candidate).cloned()
                    })
                    && let Some(value) = Self::literal_bigint_value(&value)
                {
                    return Some(value);
                }
                if let Some(array_binding) =
                    self.resolve_array_binding_from_expression(&object_candidate)
                    && let Some(index) = argument_index_from_expression(property_candidate)
                    && let Some(Some(value)) = array_binding.values.get(index as usize)
                    && let Some(value) = Self::literal_bigint_value(value)
                {
                    return Some(value);
                }
            }
        }
        None
    }

    fn bigint_update_value_from_previous(
        value: &Expression,
        previous: &Expression,
    ) -> Option<Expression> {
        let Expression::Binary { op, right, .. } = value else {
            return None;
        };
        if !matches!(op, BinaryOp::Add | BinaryOp::Subtract) {
            return None;
        }
        if !matches!(right.as_ref(), Expression::Number(number) if *number == 1.0) {
            return None;
        }
        let previous_value = match previous {
            Expression::BigInt(value) => parse_static_bigint_literal(value)?,
            _ => return None,
        };
        let unit = StaticBigInt::from(1);
        let updated = match op {
            BinaryOp::Add => previous_value + unit,
            BinaryOp::Subtract => previous_value - unit,
            _ => unreachable!("filtered above"),
        };
        Some(Expression::BigInt(updated.to_string()))
    }

    fn member_assignment_value_updates_target(
        &self,
        value: &Expression,
        object: &Expression,
        property: &Expression,
    ) -> bool {
        let Expression::Binary { left, .. } = value else {
            return false;
        };
        match left.as_ref() {
            Expression::Identifier(name) if name.starts_with("__ayy_postfix_previous_") => true,
            Expression::Member {
                object: left_object,
                property: left_property,
            } => {
                let materialized_left_object = if matches!(
                    left_object.as_ref(),
                    Expression::Identifier(name) if name.starts_with("__ayy_target_object_")
                ) {
                    self.materialize_static_expression(left_object)
                } else {
                    left_object.as_ref().clone()
                };
                let materialized_left_property = self
                    .resolve_property_key_expression(left_property)
                    .unwrap_or_else(|| self.materialize_static_expression(left_property));
                let materialized_property = self
                    .resolve_property_key_expression(property)
                    .unwrap_or_else(|| self.materialize_static_expression(property));
                static_expression_matches(&materialized_left_object, object)
                    && static_expression_matches(
                        &materialized_left_property,
                        &materialized_property,
                    )
            }
            _ => false,
        }
    }

    fn static_array_slot_bigint_update_value(
        &self,
        name: &str,
        index: u32,
        object: &Expression,
        property: &Expression,
        value: &Expression,
    ) -> Option<Expression> {
        if !self.member_assignment_value_updates_target(value, object, property) {
            return None;
        }
        let previous = self
            .state
            .speculation
            .static_semantics
            .local_array_binding(name)
            .and_then(|array_binding| array_binding.values.get(index as usize))
            .and_then(|value| value.as_ref())
            .or_else(|| {
                self.backend
                    .global_semantics
                    .values
                    .array_bindings
                    .get(name)
                    .and_then(|array_binding| array_binding.values.get(index as usize))
                    .and_then(|value| value.as_ref())
            })?;
        Self::bigint_update_value_from_previous(value, previous)
    }

    pub(in crate::backend::direct_wasm) fn member_assignment_emission_value(
        &self,
        value: &Expression,
    ) -> Expression {
        if let Some(value) = self.member_assignment_bigint_update_value(value) {
            return value;
        }
        value.clone()
    }

    fn member_assignment_static_property_value(&self, value: &Expression) -> Expression {
        if let Some(value) = self.member_assignment_bigint_update_value(value) {
            return value;
        }
        self.reference_preserving_static_value_expression(value)
    }

    fn sync_internal_target_array_source_binding(
        &mut self,
        name: &str,
        array_binding: &ArrayValueBinding,
    ) {
        if !name.starts_with("__ayy_target_object_") {
            return;
        }
        let Some(Expression::Member { object, property }) = self
            .state
            .speculation
            .static_semantics
            .local_value_binding(name)
            .cloned()
            .or_else(|| self.global_value_binding(name).cloned())
        else {
            return;
        };
        let value = array_binding_static_expression(array_binding);
        self.sync_static_member_source_value(&object, &property, &value);
    }

    fn sync_static_member_source_value(
        &mut self,
        object: &Expression,
        property: &Expression,
        value: &Expression,
    ) {
        let property = self
            .resolve_property_key_expression(property)
            .unwrap_or_else(|| self.materialize_static_expression(property));
        let property = self.canonical_object_property_expression(&property);
        let mut owner_names = Vec::new();
        match object {
            Expression::Identifier(name) => {
                if let Some((resolved_name, _)) = self.resolve_current_local_binding(name) {
                    owner_names.push(resolved_name);
                }
                owner_names.push(name.clone());
            }
            Expression::This => owner_names.push("this".to_string()),
            _ => return,
        }
        owner_names.sort();
        owner_names.dedup();

        for owner_name in owner_names {
            if let Some(index) = argument_index_from_expression(&property) {
                if let Some(array_binding) = self
                    .state
                    .speculation
                    .static_semantics
                    .local_array_binding_mut(&owner_name)
                {
                    write_array_binding_static_slot(array_binding, index, value);
                }
                if let Some(array_binding) = self
                    .backend
                    .global_semantics
                    .values
                    .array_binding_mut(&owner_name)
                {
                    write_array_binding_static_slot(array_binding, index, value);
                }
                if let Some(array_binding) = self
                    .backend
                    .shared_global_semantics
                    .values
                    .array_binding_mut(&owner_name)
                {
                    write_array_binding_static_slot(array_binding, index, value);
                }
            }

            let mut updated_object_binding = None;
            if let Some(object_binding) = self
                .state
                .speculation
                .static_semantics
                .local_object_binding_mut(&owner_name)
            {
                object_binding_set_property(object_binding, property.clone(), value.clone());
                updated_object_binding = Some(object_binding.clone());
            }
            if let Some(object_binding) = self
                .backend
                .global_semantics
                .values
                .object_binding_mut(&owner_name)
            {
                object_binding_set_property(object_binding, property.clone(), value.clone());
                updated_object_binding = Some(object_binding.clone());
            }
            if let Some(object_binding) = self
                .backend
                .shared_global_semantics
                .values
                .object_binding_mut(&owner_name)
            {
                object_binding_set_property(object_binding, property.clone(), value.clone());
                updated_object_binding = Some(object_binding.clone());
            }
            if let Some(object_binding) = updated_object_binding.as_ref() {
                self.clear_runtime_object_property_shadow_static_metadata_prefix(&owner_name);
                self.sync_runtime_object_property_shadow_static_metadata_from_binding(
                    &owner_name,
                    &object_binding,
                );
            }
        }
    }

    fn static_iterator_step_member_assignment_result(
        &self,
        value: &Expression,
    ) -> Option<Expression> {
        self.resolve_static_iterator_step_assignment_value(value)
    }

    fn binding_names_share_source(left: &str, right: &str) -> bool {
        if left == right {
            return true;
        }
        let left_source = scoped_binding_source_name(left).unwrap_or(left);
        let right_source = scoped_binding_source_name(right).unwrap_or(right);
        left_source == right_source
    }

    fn expression_references_binding_name_or_source(
        &self,
        expression: &Expression,
        target_name: &str,
        depth: usize,
    ) -> bool {
        if depth > 8 {
            return false;
        }

        if let Expression::Identifier(name) = expression {
            if Self::binding_names_share_source(name, target_name) {
                return true;
            }
            if let Some(value) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
                .or_else(|| self.global_value_binding(name))
                .cloned()
                && !static_expression_matches(&value, expression)
                && self.expression_references_binding_name_or_source(&value, target_name, depth + 1)
            {
                return true;
            }
        }

        match expression {
            Expression::Member { object, property }
            | Expression::AssignMember {
                object,
                property,
                value: _,
            } => {
                self.expression_references_binding_name_or_source(object, target_name, depth + 1)
                    || self.expression_references_binding_name_or_source(
                        property,
                        target_name,
                        depth + 1,
                    )
            }
            Expression::SuperMember { property } => {
                self.expression_references_binding_name_or_source(property, target_name, depth + 1)
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => self.expression_references_binding_name_or_source(value, target_name, depth + 1),
            Expression::Binary { left, right, .. } => {
                self.expression_references_binding_name_or_source(left, target_name, depth + 1)
                    || self.expression_references_binding_name_or_source(
                        right,
                        target_name,
                        depth + 1,
                    )
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.expression_references_binding_name_or_source(condition, target_name, depth + 1)
                    || self.expression_references_binding_name_or_source(
                        then_expression,
                        target_name,
                        depth + 1,
                    )
                    || self.expression_references_binding_name_or_source(
                        else_expression,
                        target_name,
                        depth + 1,
                    )
            }
            Expression::Sequence(expressions) => expressions.iter().any(|expression| {
                self.expression_references_binding_name_or_source(
                    expression,
                    target_name,
                    depth + 1,
                )
            }),
            Expression::Call { callee, arguments }
            | Expression::New { callee, arguments }
            | Expression::SuperCall { callee, arguments } => {
                self.expression_references_binding_name_or_source(callee, target_name, depth + 1)
                    || arguments.iter().any(|argument| match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.expression_references_binding_name_or_source(
                                expression,
                                target_name,
                                depth + 1,
                            )
                        }
                    })
            }
            _ => false,
        }
    }

    fn initialize_dynamic_member_function_assignment_capture_slots(
        &mut self,
        object: &Expression,
        member_property: &Expression,
        source_property: &Expression,
        value: &Expression,
        property_candidate: &str,
    ) -> DirectResult<()> {
        let Some(LocalFunctionBinding::User(function_name)) =
            self.resolve_function_binding_from_expression(value)
        else {
            return Ok(());
        };
        let Some(capture_bindings) = self
            .backend
            .function_registry
            .analysis
            .user_function_capture_bindings
            .get(&function_name)
            .filter(|captures| !captures.is_empty())
            .cloned()
        else {
            return Ok(());
        };
        let Some(key) = self.member_function_binding_key(object, member_property) else {
            return Ok(());
        };

        let mut capture_slots = BTreeMap::new();
        for capture_name in capture_bindings.keys() {
            if !self.expression_references_binding_name_or_source(source_property, capture_name, 0)
            {
                continue;
            }

            let source_expression = Expression::String(property_candidate.to_string());
            let hidden_name = self.allocate_named_hidden_local(
                &format!(
                    "member_closure_slot_{}_{}",
                    property_candidate, capture_name
                ),
                self.infer_value_kind(&source_expression)
                    .unwrap_or(StaticValueKind::Unknown),
            );
            let hidden_local = self
                .state
                .runtime
                .locals
                .get(&hidden_name)
                .copied()
                .expect("fresh member closure capture slot local must exist");
            self.emit_numeric_expression(&source_expression)?;
            self.push_local_set(hidden_local);
            self.update_capture_slot_binding_from_expression(&hidden_name, &source_expression)?;
            self.sync_capture_slot_runtime_object_shadows_from_expression(
                &hidden_name,
                &source_expression,
            )?;
            capture_slots.insert(capture_name.clone(), hidden_name);
        }

        if capture_slots.is_empty() {
            return Ok(());
        }

        self.state
            .speculation
            .static_semantics
            .objects
            .member_function_capture_slots
            .insert(key.clone(), capture_slots.clone());
        if self.binding_key_is_global(&key) {
            self.backend
                .set_global_member_function_capture_slots(key, capture_slots);
        }

        Ok(())
    }

    fn prevent_extensions_target_matches_named_assignment_target(
        &self,
        name: &str,
        expression: &Expression,
    ) -> bool {
        match expression {
            Expression::Identifier(target_name) => target_name == name,
            Expression::This => name == "this",
            _ => self
                .resolve_bound_alias_expression(expression)
                .filter(|resolved| !static_expression_matches(resolved, expression))
                .is_some_and(|resolved| {
                    self.prevent_extensions_target_matches_named_assignment_target(name, &resolved)
                }),
        }
    }

    fn expression_statically_prevents_extensions_of_named_assignment_target(
        &self,
        name: &str,
        expression: &Expression,
    ) -> bool {
        match expression {
            Expression::Sequence(expressions) => expressions.iter().any(|expression| {
                self.expression_statically_prevents_extensions_of_named_assignment_target(
                    name, expression,
                )
            }),
            Expression::Call { callee, arguments } => {
                let Expression::Member { object, property } = callee.as_ref() else {
                    return false;
                };
                matches!(object.as_ref(), Expression::Identifier(object_name) if object_name == "Object" || object_name == "Reflect")
                    && matches!(property.as_ref(), Expression::String(property_name) if property_name == "preventExtensions")
                    && arguments.first().is_some_and(|argument| match argument {
                        CallArgument::Expression(target) | CallArgument::Spread(target) => self
                            .prevent_extensions_target_matches_named_assignment_target(
                                name, target,
                            ),
                    })
            }
            Expression::Await(expression) | Expression::Unary { expression, .. } => self
                .expression_statically_prevents_extensions_of_named_assignment_target(
                    name, expression,
                ),
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.expression_statically_prevents_extensions_of_named_assignment_target(
                    name, condition,
                ) || self.expression_statically_prevents_extensions_of_named_assignment_target(
                    name,
                    then_expression,
                ) || self.expression_statically_prevents_extensions_of_named_assignment_target(
                    name,
                    else_expression,
                )
            }
            _ => false,
        }
    }

    fn emit_named_object_nonextensible_assignment_result(
        &mut self,
        name: &str,
        property: &Expression,
        value: &Expression,
    ) -> DirectResult<bool> {
        let materialized_property = self.canonical_object_property_expression(property);
        let storage_name = self
            .resolve_current_local_binding(name)
            .map(|(resolved_name, _)| resolved_name)
            .unwrap_or_else(|| name.to_string());
        let fallback_binding =
            self.resolve_object_binding_from_expression(&Expression::Identifier(name.to_string()));
        let binding = self
            .state
            .speculation
            .static_semantics
            .local_object_binding(&storage_name)
            .or_else(|| self.backend.global_semantics.values.object_binding(name));
        let Some(binding) = binding.or(fallback_binding.as_ref()) else {
            return Ok(false);
        };
        let property_already_present = object_binding_has_property(binding, &materialized_property);
        let rhs_prevents_extensions = !property_already_present
            && self
                .expression_statically_prevents_extensions_of_named_assignment_target(name, value);
        if object_binding_can_define_property(binding, &materialized_property)
            && !rhs_prevents_extensions
        {
            return Ok(false);
        }

        if self.state.speculation.execution_context.strict_mode
            || is_private_property_name_expression(&materialized_property)
        {
            self.emit_named_error_throw("TypeError")?;
        } else {
            self.emit_numeric_expression(value)?;
        }
        Ok(true)
    }

    fn emit_dynamic_symbol_named_object_member_assignment(
        &mut self,
        name: &str,
        property: &Expression,
        value: &Expression,
    ) -> DirectResult<bool> {
        let Some(owner_name) = self.runtime_object_property_shadow_owner_name_for_identifier(name)
        else {
            let object_expression = Expression::Identifier(name.to_string());
            let Some(object_binding) =
                self.resolve_object_binding_from_expression(&object_expression)
            else {
                return Ok(false);
            };
            if !self.binding_name_is_global(name)
                && !self
                    .state
                    .speculation
                    .static_semantics
                    .has_local_object_binding(name)
            {
                let local_object_binding = self
                    .state
                    .speculation
                    .static_semantics
                    .ensure_local_object_binding(name);
                *local_object_binding = object_binding;
            }
            let Some(owner_name) =
                self.runtime_object_property_shadow_owner_name_for_identifier(name)
            else {
                return Ok(false);
            };
            return self.emit_dynamic_symbol_named_object_member_assignment_with_owner(
                name,
                property,
                value,
                &owner_name,
            );
        };
        self.emit_dynamic_symbol_named_object_member_assignment_with_owner(
            name,
            property,
            value,
            &owner_name,
        )
    }

    fn emit_dynamic_symbol_named_object_member_assignment_with_owner(
        &mut self,
        name: &str,
        property: &Expression,
        value: &Expression,
        owner_name: &str,
    ) -> DirectResult<bool> {
        let object_expression = Expression::Identifier(name.to_string());
        let Some(object_binding) = self.resolve_object_binding_from_expression(&object_expression)
        else {
            return Ok(false);
        };
        if object_binding.symbol_properties.is_empty() {
            return Ok(false);
        }

        let materialized_property = self.canonical_object_property_expression(property);
        let materialized_value = self.reference_preserving_static_value_expression(value);
        if let Some(object_binding) = self
            .state
            .speculation
            .static_semantics
            .local_object_binding_mut(name)
        {
            object_binding.runtime_symbol_properties = true;
            object_binding_set_property(
                object_binding,
                materialized_property.clone(),
                materialized_value.clone(),
            );
        }
        if self.binding_name_is_global(name) {
            let object_binding = self
                .backend
                .global_semantics
                .values
                .object_bindings
                .entry(name.to_string())
                .or_insert_with(empty_object_value_binding);
            object_binding.runtime_symbol_properties = true;
            object_binding_set_property(object_binding, materialized_property, materialized_value);
        }
        self.update_member_function_assignment_binding(&object_expression, property, value);

        let property_local = self.allocate_temp_local();
        self.emit_numeric_expression(property)?;
        self.push_local_set(property_local);
        let value_local = self.allocate_temp_local();
        self.emit_numeric_expression(value)?;
        self.push_local_set(value_local);

        let mut open_frames = 0;
        for (existing_key, _) in object_binding.symbol_properties {
            let comparison_key = self.canonical_object_property_expression(&existing_key);
            self.push_local_get(property_local);
            self.emit_numeric_expression(&comparison_key)?;
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            open_frames += 1;
            let binding =
                self.runtime_object_property_shadow_binding_by_property(owner_name, &existing_key);
            let deleted_binding = self.runtime_object_property_shadow_deleted_binding_by_property(
                owner_name,
                &existing_key,
            );
            self.push_local_get(value_local);
            self.push_global_set(binding.value_index);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(deleted_binding.value_index);
            self.push_i32_const(0);
            self.push_global_set(deleted_binding.present_index);
            self.push_i32_const(1);
            self.push_global_set(binding.present_index);
            self.state.emission.output.instructions.push(0x05);
        }

        for _ in 0..open_frames {
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }
        self.push_local_get(value_local);
        Ok(true)
    }

    fn private_method_marker_initializer_assignment(
        &self,
        object: &Expression,
        property: &Expression,
        value: &Expression,
    ) -> bool {
        let Some(LocalFunctionBinding::User(function_name)) =
            self.resolve_member_function_binding(object, property)
        else {
            return false;
        };

        matches!(value, Expression::Identifier(value_name) if value_name == &function_name)
    }

    fn current_function_parameter_name(&self, name: &str) -> bool {
        self.state
            .speculation
            .execution_context
            .current_function_declaration
            .as_ref()
            .is_some_and(|function| {
                function
                    .params
                    .iter()
                    .any(|parameter| parameter.name == name)
            })
    }

    fn private_field_initializer_owner_name(&self, name: &str) -> Option<String> {
        if name == "this"
            && let Some(Expression::Identifier(source_name)) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding("this")
        {
            return self
                .runtime_object_property_shadow_owner_name_for_identifier(source_name)
                .or_else(|| Some(source_name.clone()));
        }
        self.runtime_object_property_shadow_owner_name_for_identifier(name)
            .or_else(|| {
                (name.starts_with("__ayy_class_expr_") || name.starts_with("__ayy_class_ctor_"))
                    .then(|| name.to_string())
            })
    }

    fn named_assignment_target_is_module_namespace(&self, name: &str, object: &Expression) -> bool {
        if self
            .current_function_name()
            .is_some_and(|function_name| function_name.starts_with("__ayy_module_init_"))
            && matches!(object, Expression::Identifier(identifier) if identifier == "exports")
        {
            return false;
        }

        if self
            .resolve_object_binding_from_expression(object)
            .as_ref()
            .is_some_and(Self::object_binding_has_module_namespace_marker)
        {
            return true;
        }

        let storage_name = self
            .resolve_current_local_binding(name)
            .map(|(resolved_name, _)| resolved_name)
            .unwrap_or_else(|| name.to_string());
        self.state
            .speculation
            .static_semantics
            .local_object_binding(&storage_name)
            .or_else(|| {
                (storage_name != name)
                    .then(|| {
                        self.state
                            .speculation
                            .static_semantics
                            .local_object_binding(name)
                    })
                    .flatten()
            })
            .or_else(|| self.backend.global_semantics.values.object_binding(name))
            .is_some_and(Self::object_binding_has_module_namespace_marker)
            || match object {
                Expression::Identifier(identifier) => self
                    .resolve_identifier_object_binding_fallback(identifier)
                    .as_ref()
                    .is_some_and(Self::object_binding_has_module_namespace_marker),
                _ => false,
            }
    }

    fn emit_module_namespace_named_property_assignment_result(
        &mut self,
        name: &str,
        object: &Expression,
        property: &Expression,
        value: &Expression,
    ) -> DirectResult<bool> {
        if !self.named_assignment_target_is_module_namespace(name, object) {
            return Ok(false);
        }

        if static_property_name_from_expression(property).is_none() {
            let _ = self.emit_property_key_expression_effects(property)?;
        }

        let value_local = self.allocate_temp_local();
        self.emit_numeric_expression(value)?;
        self.push_local_set(value_local);

        if self.state.speculation.execution_context.strict_mode {
            self.emit_named_error_throw("TypeError")?;
        }
        self.push_local_get(value_local);
        Ok(true)
    }

    fn emit_non_writable_named_property_assignment_result(
        &mut self,
        object: &Expression,
        property: &Expression,
        value: &Expression,
    ) -> DirectResult<bool> {
        let materialized_property = self.canonical_object_property_expression(property);
        let Some(property_name) = static_property_name_from_expression(&materialized_property)
        else {
            return Ok(false);
        };
        let resolved_object = self
            .resolve_bound_alias_expression(object)
            .filter(|resolved| !static_expression_matches(resolved, object));
        let materialized_object = self.materialize_static_expression(object);
        let descriptor = self
            .resolve_function_property_descriptor_binding(
                object,
                resolved_object.as_ref(),
                &materialized_object,
                &property_name,
            )
            .or_else(|| {
                self.resolve_object_property_descriptor_binding(
                    object,
                    resolved_object.as_ref(),
                    &materialized_object,
                    &materialized_property,
                    Some(&property_name),
                )
            })
            .or_else(|| {
                self.resolve_inherited_object_property_descriptor_binding(
                    object,
                    &materialized_property,
                )
            })
            .or_else(|| {
                resolved_object.as_ref().and_then(|resolved| {
                    self.resolve_inherited_object_property_descriptor_binding(
                        resolved,
                        &materialized_property,
                    )
                })
            })
            .or_else(|| {
                (!static_expression_matches(&materialized_object, object)).then(|| {
                    self.resolve_inherited_object_property_descriptor_binding(
                        &materialized_object,
                        &materialized_property,
                    )
                })?
            });
        let Some(descriptor) = descriptor else {
            return Ok(false);
        };
        let accessor_without_setter = descriptor.writable.is_none()
            && (descriptor.has_get
                || descriptor.getter.is_some()
                || descriptor.has_set
                || descriptor.setter.is_some())
            && descriptor
                .setter
                .as_ref()
                .map_or(true, |setter| matches!(setter, Expression::Undefined));
        if descriptor.writable != Some(false) && !accessor_without_setter {
            return Ok(false);
        }

        if self.state.speculation.execution_context.strict_mode {
            self.emit_named_error_throw("TypeError")?;
        } else {
            self.emit_numeric_expression(value)?;
        }
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_private_field_initializer_add(
        &mut self,
        name: &str,
        object: &Expression,
        property: &Expression,
        value: &Expression,
    ) -> DirectResult<bool> {
        let materialized_property = self.canonical_object_property_expression(property);
        if !self
            .state
            .speculation
            .execution_context
            .private_field_initializer_block
            || !is_private_property_name_expression(&materialized_property)
        {
            return Ok(false);
        }

        let Some(owner_name) = self.private_field_initializer_owner_name(name) else {
            return Ok(false);
        };
        let binding = self.runtime_object_property_shadow_binding_by_property(
            &owner_name,
            &materialized_property,
        );
        let deleted_binding = self.runtime_object_property_shadow_deleted_binding_by_property(
            &owner_name,
            &materialized_property,
        );
        let marker_property = private_brand_marker_property_expression(&materialized_property);
        let marker_bindings = marker_property.as_ref().map(|marker_property| {
            (
                self.runtime_object_property_shadow_binding_by_property(
                    &owner_name,
                    marker_property,
                ),
                self.runtime_object_property_shadow_deleted_binding_by_property(
                    &owner_name,
                    marker_property,
                ),
            )
        });
        let marker_brand_binding = self.current_private_brand_binding_name();
        let marker_value = marker_brand_binding
            .clone()
            .map(Expression::Identifier)
            .or_else(|| marker_property.as_ref().map(|_| Expression::Bool(true)));
        let materialized_value = self.reference_preserving_static_value_expression(value);
        if let Some(object_binding) = self
            .state
            .speculation
            .static_semantics
            .local_object_binding_mut(name)
        {
            object_binding_set_property(
                object_binding,
                materialized_property.clone(),
                materialized_value.clone(),
            );
            if let (Some(marker_property), Some(marker_value)) =
                (marker_property.as_ref(), marker_value.as_ref())
            {
                object_binding_set_property(
                    object_binding,
                    marker_property.clone(),
                    marker_value.clone(),
                );
            }
        } else if owner_name.starts_with("__ayy_class_expr_")
            || owner_name.starts_with("__ayy_class_ctor_")
        {
            let object_binding = self
                .state
                .speculation
                .static_semantics
                .ensure_local_object_binding(&owner_name);
            object_binding_set_property(
                object_binding,
                materialized_property.clone(),
                materialized_value.clone(),
            );
            if let (Some(marker_property), Some(marker_value)) =
                (marker_property.as_ref(), marker_value.as_ref())
            {
                object_binding_set_property(
                    object_binding,
                    marker_property.clone(),
                    marker_value.clone(),
                );
            }
            self.state
                .speculation
                .static_semantics
                .set_local_kind(&owner_name, StaticValueKind::Function);
        }
        if self.binding_name_is_global(name)
            && let Some(object_binding) = self
                .backend
                .global_semantics
                .values
                .object_bindings
                .get_mut(name)
        {
            object_binding_set_property(
                object_binding,
                materialized_property.clone(),
                materialized_value,
            );
            if let (Some(marker_property), Some(marker_value)) =
                (marker_property.as_ref(), marker_value.as_ref())
            {
                object_binding_set_property(
                    object_binding,
                    marker_property.clone(),
                    marker_value.clone(),
                );
            }
        }
        self.update_member_function_assignment_binding(object, property, value);

        let value_local = self.allocate_temp_local();
        self.emit_numeric_expression(value)?;
        self.push_local_set(value_local);
        let marker_local = if marker_bindings.is_some() && marker_value.is_some() {
            let marker_local = self.allocate_temp_local();
            if let Some(marker_brand_binding) = marker_brand_binding.as_ref() {
                self.emit_private_brand_direct_or_synthetic_runtime_value_for_binding_name(
                    marker_brand_binding,
                )?;
            } else {
                self.push_i32_const(1);
            }
            self.push_local_set(marker_local);
            Some(marker_local)
        } else {
            None
        };
        if std::env::var_os("AYY_TRACE_PRIVATE_FIELD_INIT_VALUES").is_some() {
            eprintln!(
                "private_field_init_compile current_fn={:?} owner={owner_name} property={materialized_property:?} marker_property={marker_property:?} marker_brand={marker_brand_binding:?} binding=({}, {}) marker_binding={:?}",
                self.current_function_name(),
                binding.value_index,
                binding.present_index,
                marker_bindings.as_ref().map(|(marker_binding, _)| (
                    marker_binding.value_index,
                    marker_binding.present_index
                )),
            );
            let present_local = self.allocate_temp_local();
            self.push_global_get(binding.present_index);
            self.push_local_set(present_local);
            self.emit_runtime_shadow_debug_print_local(
                "private_field_init_present",
                present_local,
            )?;
            if let (Some((marker_binding, _)), Some(marker_local)) =
                (marker_bindings.as_ref(), marker_local)
            {
                let marker_present_local = self.allocate_temp_local();
                self.push_global_get(marker_binding.present_index);
                self.push_local_set(marker_present_local);
                self.emit_runtime_shadow_debug_print_local(
                    "private_field_init_marker_present",
                    marker_present_local,
                )?;
                let marker_value_local = self.allocate_temp_local();
                self.push_global_get(marker_binding.value_index);
                self.push_local_set(marker_value_local);
                self.emit_runtime_shadow_debug_print_local(
                    "private_field_init_marker_value",
                    marker_value_local,
                )?;
                self.emit_runtime_shadow_debug_print_local(
                    "private_field_init_marker_local",
                    marker_local,
                )?;
            }
        }
        self.push_global_get(binding.present_index);
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        if let (Some((marker_binding, _)), Some(marker_local)) =
            (marker_bindings.as_ref(), marker_local)
        {
            self.push_global_get(marker_binding.present_index);
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.push_global_get(marker_binding.value_index);
            self.push_local_get(marker_local);
            self.push_binary_op(BinaryOp::Equal)?;
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
            self.state.emission.output.instructions.push(0x05);
            self.emit_named_error_throw("TypeError")?;
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        } else {
            self.emit_named_error_throw("TypeError")?;
        }
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.push_local_get(value_local);
        self.push_global_set(binding.value_index);
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_global_set(deleted_binding.value_index);
        self.push_i32_const(0);
        self.push_global_set(deleted_binding.present_index);
        self.push_i32_const(1);
        self.push_global_set(binding.present_index);
        if let (Some((marker_binding, marker_deleted_binding)), Some(marker_local)) =
            (marker_bindings.as_ref(), marker_local)
        {
            self.push_local_get(marker_local);
            self.push_global_set(marker_binding.value_index);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(marker_deleted_binding.value_index);
            self.push_i32_const(0);
            self.push_global_set(marker_deleted_binding.present_index);
            self.push_i32_const(1);
            self.push_global_set(marker_binding.present_index);
        }

        self.push_local_get(value_local);
        Ok(true)
    }

    fn emit_dynamic_string_named_object_member_assignment(
        &mut self,
        name: &str,
        property: &Expression,
        value: &Expression,
    ) -> DirectResult<bool> {
        if self.resolve_property_key_expression(property).is_some() {
            return Ok(false);
        }

        let object_expression = Expression::Identifier(name.to_string());
        let Some(object_binding) = self.resolve_object_binding_from_expression(&object_expression)
        else {
            return Ok(false);
        };
        if object_binding.string_properties.is_empty()
            && object_binding.symbol_properties.is_empty()
        {
            return Ok(false);
        }

        let Some(owner_name) = self.runtime_object_property_shadow_owner_name_for_identifier(name)
        else {
            return Ok(false);
        };

        let materialized_value = self.reference_preserving_static_value_expression(value);
        let existing_string_keys = object_binding
            .string_properties
            .iter()
            .map(|(property_name, _)| Expression::String(property_name.clone()))
            .collect::<Vec<_>>();
        let existing_symbol_keys = object_binding
            .symbol_properties
            .iter()
            .map(|(property, _)| property.clone())
            .collect::<Vec<_>>();
        let updates_symbols = !existing_symbol_keys.is_empty();

        if let Some(object_binding) = self
            .state
            .speculation
            .static_semantics
            .local_object_binding_mut(name)
        {
            if updates_symbols {
                object_binding.runtime_symbol_properties = true;
            }
            for existing_key in existing_string_keys
                .iter()
                .chain(existing_symbol_keys.iter())
            {
                object_binding_set_property(
                    object_binding,
                    existing_key.clone(),
                    materialized_value.clone(),
                );
            }
        }
        if self.binding_name_is_global(name) {
            let object_binding = self
                .backend
                .global_semantics
                .values
                .object_bindings
                .entry(name.to_string())
                .or_insert_with(empty_object_value_binding);
            if updates_symbols {
                object_binding.runtime_symbol_properties = true;
            }
            for existing_key in existing_string_keys
                .iter()
                .chain(existing_symbol_keys.iter())
            {
                object_binding_set_property(
                    object_binding,
                    existing_key.clone(),
                    materialized_value.clone(),
                );
            }
        }
        self.update_member_function_assignment_binding(&object_expression, property, value);

        let property_local = self.allocate_temp_local();
        self.emit_numeric_expression(property)?;
        self.push_local_set(property_local);
        let value_local = self.allocate_temp_local();
        self.emit_numeric_expression(value)?;
        self.push_local_set(value_local);
        self.emit_runtime_object_dynamic_property_shadow_store_from_locals(
            &owner_name,
            property_local,
            value_local,
        );

        let mut open_frames = 0;
        for existing_key in existing_string_keys {
            self.emit_runtime_property_key_match_from_local(property_local, &existing_key)?;
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            open_frames += 1;
            let binding =
                self.runtime_object_property_shadow_binding_by_property(&owner_name, &existing_key);
            let deleted_binding = self.runtime_object_property_shadow_deleted_binding_by_property(
                &owner_name,
                &existing_key,
            );
            self.push_local_get(value_local);
            self.push_global_set(binding.value_index);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(deleted_binding.value_index);
            self.push_i32_const(0);
            self.push_global_set(deleted_binding.present_index);
            self.push_i32_const(1);
            self.push_global_set(binding.present_index);
            self.state.emission.output.instructions.push(0x05);
        }
        for existing_key in existing_symbol_keys {
            let comparison_key = self.canonical_object_property_expression(&existing_key);
            self.push_local_get(property_local);
            self.emit_numeric_expression(&comparison_key)?;
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            open_frames += 1;
            let binding =
                self.runtime_object_property_shadow_binding_by_property(&owner_name, &existing_key);
            let deleted_binding = self.runtime_object_property_shadow_deleted_binding_by_property(
                &owner_name,
                &existing_key,
            );
            self.push_local_get(value_local);
            self.push_global_set(binding.value_index);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(deleted_binding.value_index);
            self.push_i32_const(0);
            self.push_global_set(deleted_binding.present_index);
            self.push_i32_const(1);
            self.push_global_set(binding.present_index);
            self.state.emission.output.instructions.push(0x05);
        }

        for _ in 0..open_frames {
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }
        self.push_local_get(value_local);
        Ok(true)
    }

    fn emit_dynamic_named_object_property_shadow_store_if_needed(
        &mut self,
        name: &str,
        property: &Expression,
        value_local: u32,
    ) -> DirectResult<()> {
        if self.resolve_property_key_expression(property).is_some()
            || !inline_summary_side_effect_free_expression(property)
        {
            return Ok(());
        }
        let Some(owner_name) = self.runtime_object_property_shadow_owner_name_for_identifier(name)
        else {
            return Ok(());
        };
        let property_local = self.allocate_temp_local();
        self.emit_numeric_expression(property)?;
        self.push_local_set(property_local);
        self.emit_runtime_object_dynamic_property_shadow_store_from_locals(
            &owner_name,
            property_local,
            value_local,
        );
        Ok(())
    }

    fn emit_private_parameter_member_assignment(
        &mut self,
        name: &str,
        object: &Expression,
        property: &Expression,
        value: &Expression,
    ) -> DirectResult<bool> {
        let materialized_property = self.canonical_object_property_expression(property);
        if !self.current_function_parameter_name(name)
            || !is_private_property_name_expression(&materialized_property)
        {
            return Ok(false);
        }

        let Some(owner_name) = self.runtime_object_property_shadow_owner_name_for_identifier(name)
        else {
            return Ok(false);
        };

        let binding = self.runtime_object_property_shadow_binding_by_property(
            &owner_name,
            &materialized_property,
        );
        let deleted_binding = self.runtime_object_property_shadow_deleted_binding_by_property(
            &owner_name,
            &materialized_property,
        );
        let accessor_without_setter = self
            .resolve_member_setter_binding(&Expression::This, &materialized_property)
            .is_none()
            && (self
                .resolve_member_getter_binding(&Expression::This, &materialized_property)
                .is_some()
                || self
                    .resolve_member_function_binding(&Expression::This, &materialized_property)
                    .is_some());
        let value_local = self.allocate_temp_local();
        self.emit_numeric_expression(value)?;
        self.push_local_set(value_local);
        if accessor_without_setter {
            self.emit_named_error_throw("TypeError")?;
            self.push_local_get(value_local);
            return Ok(true);
        }

        self.push_global_get(deleted_binding.present_index);
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.emit_named_error_throw("TypeError")?;
        self.state.emission.output.instructions.push(0x05);
        self.push_global_get(binding.present_index);
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.push_local_get(value_local);
        self.push_global_set(binding.value_index);
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_global_set(deleted_binding.value_index);
        self.push_i32_const(0);
        self.push_global_set(deleted_binding.present_index);
        self.push_i32_const(1);
        self.push_global_set(binding.present_index);
        self.state.emission.output.instructions.push(0x05);
        self.emit_named_error_throw("TypeError")?;
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();

        self.clear_runtime_object_property_shadow_deleted_binding(object, &materialized_property);
        self.push_local_get(value_local);
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_named_object_member_assignment(
        &mut self,
        object: &Expression,
        property: &Expression,
        value: &Expression,
    ) -> DirectResult<bool> {
        let trace_member_assignment = std::env::var_os("AYY_TRACE_MEMBER_ASSIGNMENT").is_some();
        if trace_member_assignment {
            eprintln!(
                "named_member_assignment:start object={object:?} property={property:?} value={value:?}"
            );
        }
        if matches!(object, Expression::This) {
            self.seed_local_this_object_binding();
        }

        if let Expression::Member {
            object: prototype_object,
            property: target_property,
        } = object
            && matches!(target_property.as_ref(), Expression::String(name) if name == "prototype")
        {
            let materialized_property = self.canonical_object_property_expression(property);
            let materialized = self.member_assignment_static_property_value(value);
            if let Expression::Identifier(name) = prototype_object.as_ref() {
                if let Some(object_binding) = self
                    .state
                    .speculation
                    .static_semantics
                    .objects
                    .local_prototype_object_bindings
                    .get_mut(name)
                {
                    object_binding_set_property(
                        object_binding,
                        materialized_property.clone(),
                        materialized.clone(),
                    );
                }
                if self.binding_name_is_global(name) {
                    let object_binding = self
                        .backend
                        .global_semantics
                        .values
                        .prototype_object_bindings
                        .entry(name.clone())
                        .or_insert_with(empty_object_value_binding);
                    object_binding_set_property(
                        object_binding,
                        materialized_property,
                        materialized,
                    );
                }
                self.update_member_function_assignment_binding(object, property, value);
                if self
                    .resolve_property_key_coercion_binding(property)
                    .is_some()
                {
                    self.emit_property_key_expression_effects(property)?;
                }
                self.emit_numeric_expression(value)?;
                return Ok(true);
            }

            if let Some((realm_id, constructor_name)) =
                self.test262_realm_constructor_member(prototype_object)
            {
                let mut constructor_binding =
                    self.test262_realm_constructor_object_binding(realm_id, &constructor_name);
                let prototype_property = Expression::String("prototype".to_string());
                let mut prototype_binding =
                    object_binding_lookup_value(&constructor_binding, &prototype_property)
                        .cloned()
                        .and_then(|value| self.resolve_object_binding_from_expression(&value))
                        .unwrap_or_else(empty_object_value_binding);
                object_binding_set_property(
                    &mut prototype_binding,
                    materialized_property,
                    materialized,
                );
                object_binding_set_property(
                    &mut constructor_binding,
                    prototype_property,
                    object_binding_to_expression(&prototype_binding),
                );
                if let Some(realm) = self.test262_realm_mut(realm_id) {
                    object_binding_set_property(
                        &mut realm.global_object_binding,
                        Expression::String(constructor_name),
                        object_binding_to_expression(&constructor_binding),
                    );
                }
                self.update_member_function_assignment_binding(object, property, value);
                if self
                    .resolve_property_key_coercion_binding(property)
                    .is_some()
                {
                    self.emit_property_key_expression_effects(property)?;
                }
                self.emit_numeric_expression(value)?;
                return Ok(true);
            }
        }

        let name = match object {
            Expression::Identifier(name) => name.as_str(),
            Expression::This => "this",
            _ => return Ok(false),
        };
        if trace_member_assignment {
            eprintln!("named_member_assignment:name name={name}");
        }

        if trace_member_assignment {
            eprintln!("named_member_assignment:static_array_property:start");
        }
        let static_array_property = if inline_summary_side_effect_free_expression(property)
            && !self.expression_depends_on_active_loop_assignment(property)
        {
            self.resolve_property_key_expression(property)
                .unwrap_or_else(|| self.materialize_static_expression(property))
        } else {
            property.clone()
        };
        if trace_member_assignment {
            eprintln!(
                "named_member_assignment:static_array_property:done property={static_array_property:?}"
            );
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:materialized_property:start");
        }
        let materialized_property = self.canonical_object_property_expression(property);
        if trace_member_assignment {
            eprintln!(
                "named_member_assignment:materialized_property:done property={materialized_property:?}"
            );
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:private_checks:start");
        }
        let private_accessor_without_setter =
            is_private_property_name_expression(&materialized_property)
                && !self.private_method_marker_initializer_assignment(
                    object,
                    &materialized_property,
                    value,
                )
                && self
                    .resolve_member_setter_binding(object, &materialized_property)
                    .is_none()
                && (self
                    .resolve_member_getter_binding(object, &materialized_property)
                    .is_some()
                    || self
                        .resolve_member_function_binding(object, &materialized_property)
                        .is_some());
        let private_data_field_assignment =
            is_private_property_name_expression(&materialized_property)
                && !self.private_method_marker_initializer_assignment(
                    object,
                    &materialized_property,
                    value,
                )
                && self
                    .resolve_member_setter_binding(object, &materialized_property)
                    .is_none()
                && self
                    .resolve_member_getter_binding(object, &materialized_property)
                    .is_none()
                && self
                    .resolve_member_function_binding(object, &materialized_property)
                    .is_none();
        if trace_member_assignment {
            eprintln!("named_member_assignment:private_checks:done");
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:private_initializer:start");
        }
        if self.emit_private_field_initializer_add(name, object, &materialized_property, value)? {
            if trace_member_assignment {
                eprintln!("named_member_assignment:private_initializer:hit");
            }
            return Ok(true);
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:private_initializer:done");
        }
        if private_accessor_without_setter {
            let value_local = self.allocate_temp_local();
            self.emit_numeric_expression(value)?;
            self.push_local_set(value_local);
            self.emit_named_error_throw("TypeError")?;
            self.push_local_get(value_local);
            return Ok(true);
        }
        if private_data_field_assignment {
            self.emit_private_member_assignment_target_base_or_throw(object)?;
            let value_local = self.allocate_temp_local();
            self.emit_numeric_expression(value)?;
            self.push_local_set(value_local);
            self.emit_private_data_field_brand_check_after_base_or_throw(
                object,
                &materialized_property,
            )?;

            let materialized = self.materialize_static_expression(value);
            if let Some(object_binding) = self
                .state
                .speculation
                .static_semantics
                .local_object_binding_mut(name)
            {
                object_binding_set_property(
                    object_binding,
                    materialized_property.clone(),
                    materialized.clone(),
                );
            }
            if self.binding_name_is_global(name)
                && let Some(object_binding) = self
                    .backend
                    .global_semantics
                    .values
                    .object_bindings
                    .get_mut(name)
            {
                object_binding_set_property(
                    object_binding,
                    materialized_property.clone(),
                    materialized,
                );
            }
            self.update_member_function_assignment_binding(object, property, value);
            if let Expression::String(property_name) = &materialized_property {
                self.emit_scoped_property_store_from_local(
                    object,
                    property_name,
                    value_local,
                    value,
                )?;
                self.clear_runtime_object_property_shadow_deleted_binding(
                    object,
                    &materialized_property,
                );
                return Ok(true);
            }
            self.push_local_get(value_local);
            return Ok(true);
        }

        if trace_member_assignment {
            eprintln!("named_member_assignment:typed_array:start");
        }
        if self
            .state
            .speculation
            .static_semantics
            .has_local_typed_array_view_binding(name)
        {
            self.emit_typed_array_view_write(name, property, value)?;
            return Ok(true);
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:typed_array:done");
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:length:start");
        }
        if matches!(static_array_property, Expression::String(ref property_name) if property_name == "length")
            && let Some(new_length_number) = self.resolve_static_number_value(value)
            && new_length_number.is_finite()
            && new_length_number >= 0.0
            && new_length_number.fract() == 0.0
        {
            let new_length = new_length_number as usize;
            let length_local = self
                .state
                .speculation
                .static_semantics
                .runtime_array_length_local(name);
            let use_global_runtime_array = self.is_named_global_array_binding(name)
                && (!self.state.speculation.execution_context.top_level_function
                    || self.uses_global_runtime_array_state(name));
            let value_local = self.allocate_temp_local();
            self.emit_numeric_expression(value)?;
            self.push_local_set(value_local);
            let mut old_length = None;
            let mut synced_array_binding = None;
            if let Some(array_binding) = self
                .state
                .speculation
                .static_semantics
                .local_array_binding_mut(name)
            {
                old_length = Some(array_binding.values.len());
                array_binding.values.truncate(new_length);
                synced_array_binding = Some(array_binding.clone());
            } else if let Some(array_binding) = self
                .backend
                .global_semantics
                .values
                .array_bindings
                .get_mut(name)
            {
                old_length = Some(array_binding.values.len());
                array_binding.values.truncate(new_length);
                synced_array_binding = Some(array_binding.clone());
            }
            if let Some(old_length) = old_length {
                if self.binding_name_is_global(name) {
                    self.backend
                        .sync_global_array_binding(name, synced_array_binding.clone());
                }
                for index in new_length..old_length.min(TRACKED_ARRAY_SLOT_LIMIT as usize) {
                    if use_global_runtime_array {
                        self.clear_global_runtime_array_slot(name, index as u32);
                    } else {
                        self.clear_runtime_array_slot(name, index as u32);
                    }
                }
                if !use_global_runtime_array && let Some(length_local) = length_local {
                    self.push_i32_const(new_length as i32);
                    self.push_local_set(length_local);
                }
                if use_global_runtime_array {
                    self.emit_global_runtime_array_length_write(name, new_length as i32);
                }
                self.push_local_get(value_local);
                return Ok(true);
            }
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:length:done");
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:realm:start");
        }
        if let Some(realm_id) = self.resolve_test262_realm_global_id_from_expression(object) {
            let materialized_property = self.canonical_object_property_expression(property);
            let materialized = self.materialize_static_expression(value);
            if let Some(realm) = self.test262_realm_mut(realm_id) {
                object_binding_set_property(
                    &mut realm.global_object_binding,
                    materialized_property,
                    materialized,
                );
                self.emit_numeric_expression(value)?;
                return Ok(true);
            }
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:realm:done");
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:module_namespace:start");
        }
        if self
            .emit_module_namespace_named_property_assignment_result(name, object, property, value)?
        {
            if trace_member_assignment {
                eprintln!("named_member_assignment:module_namespace:hit");
            }
            return Ok(true);
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:module_namespace:done");
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:index:start");
        }
        if let Some(index) = argument_index_from_expression(&static_array_property) {
            let emitted_value = self
                .static_array_slot_bigint_update_value(name, index, object, property, value)
                .unwrap_or_else(|| self.member_assignment_emission_value(value));
            let materialized = self.member_assignment_static_property_value(&emitted_value);
            let length_local = self
                .state
                .speculation
                .static_semantics
                .runtime_array_length_local(name);
            let use_global_runtime_array = self.is_named_global_array_binding(name)
                && (!self.state.speculation.execution_context.top_level_function
                    || self.uses_global_runtime_array_state(name));
            let value_local = self.allocate_temp_local();
            self.emit_numeric_expression(&emitted_value)?;
            self.push_local_set(value_local);
            let mut array_length = None;
            let mut updated_array_binding = None;
            if let Some(array_binding) = self
                .state
                .speculation
                .static_semantics
                .local_array_binding_mut(name)
            {
                while array_binding.values.len() <= index as usize {
                    array_binding.values.push(None);
                }
                array_binding.values[index as usize] = Some(materialized.clone());
                array_length = Some(array_binding.values.len() as i32);
                updated_array_binding = Some(array_binding.clone());
            } else if let Some(array_binding) = self
                .backend
                .global_semantics
                .values
                .array_bindings
                .get_mut(name)
            {
                while array_binding.values.len() <= index as usize {
                    array_binding.values.push(None);
                }
                array_binding.values[index as usize] = Some(materialized);
                array_length = Some(array_binding.values.len() as i32);
                updated_array_binding = Some(array_binding.clone());
            }
            if let Some(array_length) = array_length {
                if let Some(updated_array_binding) = updated_array_binding.as_ref() {
                    self.sync_internal_target_array_source_binding(name, updated_array_binding);
                }
                self.update_tracked_array_specialized_function_value(name, index, &emitted_value)?;
                if !use_global_runtime_array && let Some(length_local) = length_local {
                    self.push_i32_const(array_length);
                    self.push_local_set(length_local);
                }
                if use_global_runtime_array {
                    if self.emit_global_runtime_array_slot_write_from_local(
                        name,
                        index,
                        value_local,
                    )? {
                        self.state.emission.output.instructions.push(0x1a);
                    }
                } else {
                    if length_local.is_some()
                        || self
                            .state
                            .speculation
                            .static_semantics
                            .has_runtime_array_slots(name)
                    {
                        self.ensure_runtime_array_slot_entry(name, index);
                    }
                    if self.emit_runtime_array_slot_write_from_local(name, index, value_local)? {
                        self.state.emission.output.instructions.push(0x1a);
                    }
                }
                self.push_local_get(value_local);
                return Ok(true);
            }
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:index:done");
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:dynamic_array:start");
        }
        if !static_array_property_is_known_non_index(&static_array_property) {
            if self.is_named_global_array_binding(name)
                && (!self.state.speculation.execution_context.top_level_function
                    || self.uses_global_runtime_array_state(name))
            {
                if self.emit_dynamic_global_runtime_array_slot_write(name, property, value)? {
                    return Ok(true);
                }
            } else if self.emit_dynamic_runtime_array_slot_write(name, property, value)? {
                return Ok(true);
            }
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:dynamic_array:done");
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:nonextensible:start");
        }
        if self.emit_named_object_nonextensible_assignment_result(name, property, value)? {
            return Ok(true);
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:nonextensible:done");
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:nonwritable:start");
        }
        if self.emit_non_writable_named_property_assignment_result(object, property, value)? {
            return Ok(true);
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:nonwritable:done");
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:resolved_property:start");
        }
        let resolved_property = if self.expression_depends_on_active_loop_assignment(property) {
            match property {
                Expression::Identifier(name) if name.starts_with("__ayy_target_property_") => self
                    .resolve_property_key_expression(property)
                    .unwrap_or_else(|| self.materialize_static_expression(property)),
                _ => self.materialize_static_expression(property),
            }
        } else {
            self.resolve_property_key_expression(property)
                .unwrap_or_else(|| self.materialize_static_expression(property))
        };
        if trace_member_assignment {
            eprintln!(
                "named_member_assignment:resolved_property:done property={resolved_property:?}"
            );
        }
        let value_references_internal_iterator_step =
            member_assignment_expression_references_internal_iterator_step(value);
        if matches!(&resolved_property, Expression::String(property_name) if property_name == "prototype")
            && self.expression_aliases_named_member_property(value, name, "prototype", 0)
            && self
                .resolve_function_binding_from_expression(object)
                .is_some()
        {
            if let Some(object_binding) = self
                .state
                .speculation
                .static_semantics
                .local_object_binding_mut(name)
            {
                object_binding_remove_property(object_binding, &resolved_property);
            }
            if let Some(object_binding) = self
                .backend
                .global_semantics
                .values
                .object_bindings
                .get_mut(name)
            {
                object_binding_remove_property(object_binding, &resolved_property);
            }
            self.clear_runtime_object_property_shadow_deleted_binding(object, &resolved_property);
            self.emit_numeric_expression(value)?;
            return Ok(true);
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:function_object:start");
        }
        if self
            .resolve_function_binding_from_expression(object)
            .is_some()
        {
            let materialized = self.member_assignment_static_property_value(value);
            if matches!(&resolved_property, Expression::String(property_name) if property_name == "prototype")
            {
                self.update_prototype_object_binding(name, value);
            }
            if self.binding_name_is_global(name) {
                let object_binding = self
                    .backend
                    .global_semantics
                    .values
                    .object_bindings
                    .entry(name.to_string())
                    .or_insert_with(empty_object_value_binding);
                object_binding_set_property(
                    object_binding,
                    resolved_property.clone(),
                    materialized.clone(),
                );
            } else {
                let object_binding = self
                    .state
                    .speculation
                    .static_semantics
                    .ensure_local_object_binding(name);
                object_binding_set_property(
                    object_binding,
                    resolved_property.clone(),
                    materialized.clone(),
                );
            }
            self.clear_runtime_object_property_shadow_deleted_binding(object, &resolved_property);
            self.update_member_function_assignment_binding(object, property, value);
            if name.starts_with("__ayy_class_expr_") || name.starts_with("__ayy_class_ctor_") {
                let owner_name = name.to_string();
                let binding = self.runtime_object_property_shadow_binding_by_property(
                    &owner_name,
                    &resolved_property,
                );
                let deleted_binding = self
                    .runtime_object_property_shadow_deleted_binding_by_property(
                        &owner_name,
                        &resolved_property,
                    );
                let value_local = self.allocate_temp_local();
                self.emit_numeric_expression(value)?;
                self.push_local_set(value_local);
                self.push_local_get(value_local);
                self.push_global_set(binding.value_index);
                self.push_i32_const(JS_UNDEFINED_TAG);
                self.push_global_set(deleted_binding.value_index);
                self.push_i32_const(0);
                self.push_global_set(deleted_binding.present_index);
                self.push_i32_const(1);
                self.push_global_set(binding.present_index);
                self.push_local_get(value_local);
                return Ok(true);
            }
            self.emit_numeric_expression(value)?;
            return Ok(true);
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:function_object:done");
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:array_object:start");
        }
        if self
            .state
            .speculation
            .static_semantics
            .has_local_array_binding(name)
            || self
                .backend
                .global_semantics
                .values
                .array_bindings
                .contains_key(name)
        {
            let materialized = self.materialize_static_expression(value);
            if self
                .state
                .speculation
                .static_semantics
                .has_local_array_binding(name)
            {
                let object_binding = self
                    .state
                    .speculation
                    .static_semantics
                    .ensure_local_object_binding(name);
                object_binding_set_property(
                    object_binding,
                    resolved_property.clone(),
                    materialized.clone(),
                );
            }
            if self
                .backend
                .global_semantics
                .values
                .array_bindings
                .contains_key(name)
            {
                let object_binding = self
                    .backend
                    .global_semantics
                    .values
                    .object_bindings
                    .entry(name.to_string())
                    .or_insert_with(empty_object_value_binding);
                object_binding_set_property(
                    object_binding,
                    resolved_property.clone(),
                    materialized,
                );
            }
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:array_object:done");
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:private_parameter:start");
        }
        if self.emit_private_parameter_member_assignment(name, object, property, value)? {
            return Ok(true);
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:private_parameter:done");
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:member_function_property:start");
        }
        if self
            .member_function_binding_property(&resolved_property)
            .is_some()
            && !value_references_internal_iterator_step
        {
            self.update_member_function_assignment_binding(object, &resolved_property, value);
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:member_function_property:done");
        }
        if value_references_internal_iterator_step
            && let Some(setter_binding) =
                self.resolve_member_setter_binding(object, &resolved_property)
        {
            if trace_member_assignment {
                eprintln!("named_member_assignment:iterator_step_setter:start");
            }
            let receiver_hidden_name =
                self.allocate_named_hidden_local("setter_receiver", StaticValueKind::Unknown);
            let receiver_local = self
                .state
                .runtime
                .locals
                .get(&receiver_hidden_name)
                .copied()
                .expect("fresh setter receiver hidden local must exist");
            let value_hidden_name =
                self.allocate_named_hidden_local("setter_value", StaticValueKind::Unknown);
            let value_local = self
                .state
                .runtime
                .locals
                .get(&value_hidden_name)
                .copied()
                .expect("fresh setter value hidden local must exist");
            self.emit_numeric_expression(object)?;
            self.push_local_set(receiver_local);
            self.emit_numeric_expression(value)?;
            self.push_local_set(value_local);
            let _ = self.emit_property_key_expression_effects(property)?;
            let receiver_expression = Expression::Identifier(receiver_hidden_name.clone());
            self.update_local_value_binding(&receiver_hidden_name, object);
            self.update_local_object_binding(&receiver_hidden_name, object);
            if self.emit_function_binding_call_with_function_this_binding_from_argument_locals(
                &setter_binding,
                &[value_local],
                1,
                &receiver_expression,
            )? {
                self.state.emission.output.instructions.push(0x1a);
            }
            self.push_local_get(value_local);
            if trace_member_assignment {
                eprintln!("named_member_assignment:iterator_step_setter:done");
            }
            return Ok(true);
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:scoped_shadow:start");
        }
        if !value_references_internal_iterator_step
            && let Expression::String(property_name) = &resolved_property
            && self
                .runtime_object_property_shadow_owner_name_for_identifier(name)
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
            self.clear_runtime_object_property_shadow_deleted_binding(
                object,
                &Expression::String(property_name.clone()),
            );
            return Ok(true);
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:scoped_shadow:done");
        }
        if self.named_member_assignment_property_requires_runtime_coercion(property) {
            self.invalidate_dynamic_named_member_assignment_target(name);
            if Self::assignment_value_reads_same_member(value, object, property) {
                self.emit_numeric_expression(value)?;
                return Ok(true);
            }
            return Ok(false);
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:dynamic_string:start");
        }
        if !value_references_internal_iterator_step
            && self.emit_dynamic_string_named_object_member_assignment(name, property, value)?
        {
            return Ok(true);
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:dynamic_string:done");
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:dynamic_symbol:start");
        }
        if !value_references_internal_iterator_step
            && self.emit_dynamic_symbol_named_object_member_assignment(name, property, value)?
        {
            return Ok(true);
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:dynamic_symbol:done");
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:dynamic_property_candidates:start");
        }
        let dynamic_property_candidates = if value_references_internal_iterator_step {
            Vec::new()
        } else {
            self.dynamic_for_in_key_property_candidates(property)
        };
        if trace_member_assignment {
            eprintln!(
                "named_member_assignment:dynamic_property_candidates:done count={}",
                dynamic_property_candidates.len()
            );
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:materialized_value:start");
        }
        let materialized = if value_references_internal_iterator_step {
            self.static_iterator_step_member_assignment_result(value)
                .unwrap_or_else(|| value.clone())
        } else {
            self.member_assignment_static_property_value(value)
        };
        if trace_member_assignment {
            eprintln!("named_member_assignment:materialized_value:done value={materialized:?}");
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:local_object_binding:start");
        }
        if let Some(object_binding) = self
            .state
            .speculation
            .static_semantics
            .local_object_binding_mut(name)
        {
            if dynamic_property_candidates.is_empty() {
                object_binding_set_property(
                    object_binding,
                    resolved_property.clone(),
                    materialized.clone(),
                );
            } else {
                for property_name in &dynamic_property_candidates {
                    object_binding_set_property(
                        object_binding,
                        Expression::String(property_name.clone()),
                        materialized.clone(),
                    );
                }
            }
            self.clear_runtime_object_property_shadow_deleted_binding(object, &resolved_property);
            if dynamic_property_candidates.is_empty() && !value_references_internal_iterator_step {
                self.update_member_function_assignment_binding(object, property, value);
            } else {
                for property_name in &dynamic_property_candidates {
                    let property_expression = Expression::String(property_name.clone());
                    self.update_member_function_assignment_binding(
                        object,
                        &property_expression,
                        value,
                    );
                    self.initialize_dynamic_member_function_assignment_capture_slots(
                        object,
                        &property_expression,
                        property,
                        value,
                        property_name,
                    )?;
                }
            }
            let value_local = self.allocate_temp_local();
            if dynamic_property_candidates.is_empty()
                && !value_references_internal_iterator_step
                && self
                    .resolve_property_key_coercion_binding(property)
                    .is_some()
            {
                self.emit_property_key_expression_effects(property)?;
            }
            self.emit_numeric_expression(value)?;
            self.push_local_set(value_local);
            if dynamic_property_candidates.is_empty() && !value_references_internal_iterator_step {
                self.sync_closure_capture_slots_from_member_store(
                    object,
                    &resolved_property,
                    value_local,
                    value,
                )?;
                self.emit_dynamic_named_object_property_shadow_store_if_needed(
                    name,
                    property,
                    value_local,
                )?;
            } else if !value_references_internal_iterator_step {
                for property_name in &dynamic_property_candidates {
                    self.sync_closure_capture_slots_from_member_store(
                        object,
                        &Expression::String(property_name.clone()),
                        value_local,
                        value,
                    )?;
                }
            }
            self.push_local_get(value_local);
            return Ok(true);
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:local_object_binding:done");
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:global_object_binding:start");
        }
        if let Some(object_binding) = self
            .backend
            .global_semantics
            .values
            .object_bindings
            .get_mut(name)
        {
            if dynamic_property_candidates.is_empty() {
                object_binding_set_property(
                    object_binding,
                    resolved_property.clone(),
                    materialized,
                );
            } else {
                for property_name in &dynamic_property_candidates {
                    object_binding_set_property(
                        object_binding,
                        Expression::String(property_name.clone()),
                        materialized.clone(),
                    );
                }
            }
            self.clear_runtime_object_property_shadow_deleted_binding(object, &resolved_property);
            if dynamic_property_candidates.is_empty() && !value_references_internal_iterator_step {
                self.update_member_function_assignment_binding(object, property, value);
            } else {
                for property_name in &dynamic_property_candidates {
                    let property_expression = Expression::String(property_name.clone());
                    self.update_member_function_assignment_binding(
                        object,
                        &property_expression,
                        value,
                    );
                    self.initialize_dynamic_member_function_assignment_capture_slots(
                        object,
                        &property_expression,
                        property,
                        value,
                        property_name,
                    )?;
                }
            }
            let value_local = self.allocate_temp_local();
            if dynamic_property_candidates.is_empty()
                && !value_references_internal_iterator_step
                && self
                    .resolve_property_key_coercion_binding(property)
                    .is_some()
            {
                self.emit_property_key_expression_effects(property)?;
            }
            self.emit_numeric_expression(value)?;
            self.push_local_set(value_local);
            if dynamic_property_candidates.is_empty() && !value_references_internal_iterator_step {
                self.sync_closure_capture_slots_from_member_store(
                    object,
                    &resolved_property,
                    value_local,
                    value,
                )?;
                self.emit_dynamic_named_object_property_shadow_store_if_needed(
                    name,
                    property,
                    value_local,
                )?;
            } else if !value_references_internal_iterator_step {
                for property_name in &dynamic_property_candidates {
                    self.sync_closure_capture_slots_from_member_store(
                        object,
                        &Expression::String(property_name.clone()),
                        value_local,
                        value,
                    )?;
                }
            }
            self.push_local_get(value_local);
            return Ok(true);
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:global_object_binding:done");
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:final_function_object:start");
        }
        if self
            .resolve_function_binding_from_expression(object)
            .is_some()
        {
            let object_binding = self
                .state
                .speculation
                .static_semantics
                .ensure_local_object_binding(name);
            object_binding_set_property(
                object_binding,
                resolved_property.clone(),
                materialized.clone(),
            );
            if self.binding_name_is_global(name) {
                let global_binding = self
                    .backend
                    .global_semantics
                    .values
                    .object_bindings
                    .entry(name.to_string())
                    .or_insert_with(empty_object_value_binding);
                object_binding_set_property(
                    global_binding,
                    resolved_property.clone(),
                    materialized,
                );
            }
            self.clear_runtime_object_property_shadow_deleted_binding(object, &resolved_property);
            self.state
                .speculation
                .static_semantics
                .set_local_kind(name, StaticValueKind::Object);
            self.update_member_function_assignment_binding(object, property, value);
            self.emit_numeric_expression(value)?;
            return Ok(true);
        }
        if trace_member_assignment {
            eprintln!("named_member_assignment:final_function_object:done");
        }

        Ok(false)
    }
}
