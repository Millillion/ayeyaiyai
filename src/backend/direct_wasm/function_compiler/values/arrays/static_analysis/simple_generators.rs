use super::*;

impl<'a> FunctionCompiler<'a> {
    fn simple_generator_cached_binding_value(value: Expression) -> Expression {
        match value {
            Expression::Sequence(expressions) => expressions
                .last()
                .cloned()
                .map(Self::simple_generator_cached_binding_value)
                .unwrap_or(Expression::Undefined),
            other => other,
        }
    }

    fn simple_generator_effect_binding_value_with_index<'b>(
        &'b self,
        name: &str,
        effects: &'b [Statement],
    ) -> Option<(usize, &'b Expression, &'b [Statement])> {
        for (index, effect) in effects.iter().enumerate().rev() {
            match effect {
                Statement::Let {
                    name: binding_name,
                    value,
                    ..
                }
                | Statement::Var {
                    name: binding_name,
                    value,
                }
                | Statement::Assign {
                    name: binding_name,
                    value,
                } if binding_name == name => return Some((index, value, &effects[..index])),
                _ => {}
            }
        }
        None
    }

    fn simple_generator_effect_binding_value<'b>(
        &'b self,
        name: &str,
        effects: &'b [Statement],
    ) -> Option<(&'b Expression, &'b [Statement])> {
        self.simple_generator_effect_binding_value_with_index(name, effects)
            .map(|(_, value, prior_effects)| (value, prior_effects))
    }

    fn simple_generator_deleted_property_for_binding(
        &self,
        statement: &Statement,
        binding_name: &str,
    ) -> Option<Expression> {
        let Statement::Expression(Expression::Unary {
            op: UnaryOp::Delete,
            expression,
        }) = statement
        else {
            return None;
        };
        let Expression::Member { object, property } = expression.as_ref() else {
            return None;
        };
        if !matches!(object.as_ref(), Expression::Identifier(name) if name == binding_name) {
            return None;
        }
        Some(self.materialize_static_expression(property))
    }

    fn simple_generator_remove_static_object_property(
        &self,
        value: Expression,
        property: &Expression,
    ) -> Expression {
        let Expression::Object(entries) = value else {
            return value;
        };
        Expression::Object(
            entries
                .into_iter()
                .filter(|entry| match entry {
                    ObjectEntry::Data { key, .. }
                    | ObjectEntry::Getter { key, .. }
                    | ObjectEntry::Setter { key, .. } => {
                        self.materialize_static_expression(key) != *property
                    }
                    ObjectEntry::Spread(_) => true,
                })
                .collect(),
        )
    }

    fn simple_generator_apply_binding_deletes(
        &self,
        binding_name: &str,
        value: Expression,
        subsequent_effects: &[Statement],
    ) -> Expression {
        let mut value = Self::simple_generator_cached_binding_value(value);
        for effect in subsequent_effects {
            let Some(property) =
                self.simple_generator_deleted_property_for_binding(effect, binding_name)
            else {
                continue;
            };
            value = self.simple_generator_remove_static_object_property(value, &property);
        }
        value
    }

    fn simple_generator_effect_call_is_iterator_next(value: &Expression) -> Option<&str> {
        let Expression::Call { callee, arguments } = value else {
            return None;
        };
        if !arguments.is_empty() {
            return None;
        }
        let Expression::Member { object, property } = callee.as_ref() else {
            return None;
        };
        if !matches!(property.as_ref(), Expression::String(name) if name == "next") {
            return None;
        }
        let Expression::Identifier(iterator_name) = object.as_ref() else {
            return None;
        };
        Some(iterator_name)
    }

    fn simple_generator_effect_iterator_source_values(
        &self,
        iterator_name: &str,
        effects: &[Statement],
    ) -> Option<Vec<Option<Expression>>> {
        let (iterator_value, prior_effects) =
            self.simple_generator_effect_binding_value(iterator_name, effects)?;
        let Expression::GetIterator(source) = iterator_value else {
            return None;
        };
        let source = self
            .simple_generator_effect_expression(source, prior_effects)
            .or_else(|| match source.as_ref() {
                Expression::Identifier(name) => self.simple_generator_static_array_expression(name),
                _ => None,
            })
            .or_else(|| {
                let materialized = self.materialize_static_expression(source);
                (!static_expression_matches(&materialized, source)).then_some(materialized)
            })
            .unwrap_or_else(|| source.as_ref().clone());
        if let Expression::Identifier(name) = &source
            && let Some(values) = self.simple_generator_iterator_binding_remaining_values(name)
        {
            return Some(values);
        }
        if let Some(values) = self.simple_generator_static_iterable_prefix_values(&source) {
            return Some(values);
        }
        if let Some(source_kind) = self.resolve_iterator_source_kind(&source) {
            match source_kind {
                IteratorSourceKind::SimpleGenerator { steps, .. } => {
                    return steps
                        .into_iter()
                        .map(|step| match step.outcome {
                            SimpleGeneratorStepOutcome::Yield(value) => {
                                Some(Some(self.materialize_static_expression(
                                    &Self::substitute_sent_expression(
                                        &value,
                                        &Expression::Undefined,
                                    ),
                                )))
                            }
                            SimpleGeneratorStepOutcome::Throw(_) => None,
                        })
                        .collect();
                }
                IteratorSourceKind::StaticArray {
                    values, keys_only, ..
                } => {
                    return values
                        .into_iter()
                        .enumerate()
                        .map(|(index, value)| {
                            Some(if keys_only {
                                Some(Expression::Number(index as f64))
                            } else {
                                Some(value.unwrap_or(Expression::Undefined))
                            })
                        })
                        .collect();
                }
                _ => {}
            }
        }
        let source = match source {
            Expression::Identifier(name) => self
                .simple_generator_static_array_expression(&name)
                .unwrap_or(Expression::Identifier(name)),
            other => other,
        };
        let Expression::Array(elements) = source else {
            return None;
        };
        elements
            .into_iter()
            .map(|element| match element {
                ArrayElement::Expression(value) => Some(Some(value)),
                ArrayElement::Spread(_) => None,
            })
            .collect()
    }

    fn simple_generator_iterator_binding_remaining_values(
        &self,
        name: &str,
    ) -> Option<Vec<Option<Expression>>> {
        let binding_name = self
            .resolve_local_array_iterator_binding_name(name)
            .unwrap_or_else(|| name.to_string());
        let binding = self
            .state
            .speculation
            .static_semantics
            .local_array_iterator_binding(&binding_name)?;
        let start_index = binding.static_index.unwrap_or(0);
        match &binding.source {
            IteratorSourceKind::StaticArray {
                values, keys_only, ..
            } => values
                .iter()
                .enumerate()
                .skip(start_index)
                .map(|(index, value)| {
                    Some(if *keys_only {
                        Some(Expression::Number(index as f64))
                    } else {
                        Some(value.clone().unwrap_or(Expression::Undefined))
                    })
                })
                .collect(),
            IteratorSourceKind::SimpleGenerator { steps, .. } => steps
                .iter()
                .skip(start_index)
                .map(|step| match &step.outcome {
                    SimpleGeneratorStepOutcome::Yield(value) => {
                        Some(Some(self.materialize_static_expression(
                            &Self::substitute_sent_expression(value, &Expression::Undefined),
                        )))
                    }
                    SimpleGeneratorStepOutcome::Throw(_) => None,
                })
                .collect(),
            _ => None,
        }
    }

    fn simple_generator_static_iterable_prefix_values(
        &self,
        expression: &Expression,
    ) -> Option<Vec<Option<Expression>>> {
        let symbol_iterator = self.materialize_static_expression(&Expression::Member {
            object: Box::new(Expression::Identifier("Symbol".to_string())),
            property: Box::new(Expression::String("iterator".to_string())),
        });
        let iterator_method_binding = self
            .resolve_object_binding_from_expression(expression)
            .and_then(|object_binding| {
                object_binding_lookup_value(&object_binding, &symbol_iterator).cloned()
            })
            .and_then(|iterator_method| {
                self.resolve_function_binding_from_expression(&iterator_method)
            })
            .or_else(|| self.resolve_member_function_binding(expression, &symbol_iterator))?;
        let LocalFunctionBinding::User(iterator_function_name) = iterator_method_binding else {
            return None;
        };
        let (iterator_result, iterator_bindings) = self
            .execute_simple_static_user_function_with_bindings(
                &iterator_function_name,
                &HashMap::new(),
            )?;
        let iterator_result_binding =
            self.resolve_object_binding_from_expression(&iterator_result)?;
        let next_value = object_binding_lookup_value(
            &iterator_result_binding,
            &Expression::String("next".to_string()),
        )?;
        let LocalFunctionBinding::User(next_function_name) =
            self.resolve_function_binding_from_expression(next_value)?
        else {
            return None;
        };

        let mut step_bindings = iterator_bindings;
        let mut values = Vec::new();
        for _ in 0..256 {
            let (step_result, updated_bindings) = self
                .execute_simple_static_user_function_with_bindings(
                    &next_function_name,
                    &step_bindings,
                )?;
            step_bindings = updated_bindings;
            let step_object_binding = self.resolve_object_binding_from_expression(&step_result)?;
            let done = object_binding_lookup_value(
                &step_object_binding,
                &Expression::String("done".to_string()),
            )
            .cloned()
            .unwrap_or(Expression::Bool(false));
            let value = object_binding_lookup_value(
                &step_object_binding,
                &Expression::String("value".to_string()),
            )
            .cloned()
            .unwrap_or(Expression::Undefined);
            match done {
                Expression::Bool(true) => return Some(values),
                Expression::Bool(false) => values.push(Some(value)),
                _ => return None,
            }
        }

        (!values.is_empty()).then_some(values)
    }

    fn simple_generator_static_array_expression(&self, name: &str) -> Option<Expression> {
        let binding = self
            .state
            .speculation
            .static_semantics
            .local_array_binding(name)
            .or_else(|| {
                self.backend
                    .global_semantics
                    .values
                    .array_bindings
                    .get(name)
            })?;
        Some(Expression::Array(
            binding
                .values
                .iter()
                .map(|value| {
                    ArrayElement::Expression(value.clone().unwrap_or(Expression::Undefined))
                })
                .collect(),
        ))
    }

    fn simple_generator_effect_iterator_step_object(
        &self,
        _step_name: &str,
        value: &Expression,
        effects: &[Statement],
    ) -> Option<Expression> {
        let iterator_name = Self::simple_generator_effect_call_is_iterator_next(value)?;
        let values = self.simple_generator_effect_iterator_source_values(iterator_name, effects)?;
        let step_index = effects
            .iter()
            .filter(|effect| {
                matches!(
                    effect,
                    Statement::Let { value, .. }
                    | Statement::Var { value, .. }
                    | Statement::Assign { value, .. }
                        if Self::simple_generator_effect_call_is_iterator_next(value)
                            == Some(iterator_name)
                )
            })
            .count()
            .saturating_sub(1);
        let done = step_index >= values.len();
        let value = if done {
            Expression::Undefined
        } else {
            values
                .get(step_index)
                .cloned()
                .flatten()
                .unwrap_or(Expression::Undefined)
        };
        Some(Expression::Object(vec![
            ObjectEntry::Data {
                key: Expression::String("done".to_string()),
                value: Expression::Bool(done),
            },
            ObjectEntry::Data {
                key: Expression::String("value".to_string()),
                value,
            },
        ]))
    }

    fn simple_generator_spread_object_binding(
        &self,
        spread: &Expression,
    ) -> Option<ObjectValueBinding> {
        if let Expression::String(text) = spread {
            return Some(Self::simple_generator_string_object_binding(text));
        }
        match spread {
            Expression::Identifier(name) => self
                .resolve_runtime_shadow_object_binding(name)
                .or_else(|| self.resolve_object_binding_from_expression(spread)),
            Expression::This => self
                .resolve_runtime_shadow_object_binding("this")
                .or_else(|| self.resolve_object_binding_from_expression(spread)),
            _ => self.resolve_object_binding_from_expression(spread),
        }
    }

    fn simple_generator_string_object_binding(text: &str) -> ObjectValueBinding {
        let mut binding = empty_object_value_binding();
        for (index, character) in text.chars().enumerate() {
            object_binding_set_property(
                &mut binding,
                Expression::String(index.to_string()),
                Expression::String(character.to_string()),
            );
        }
        binding
    }

    fn simple_generator_array_push_arguments<'b>(
        statement: &'b Statement,
        name: &str,
    ) -> Option<&'b [CallArgument]> {
        let Statement::Expression(Expression::Call { callee, arguments }) = statement else {
            return None;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return None;
        };
        if matches!(object.as_ref(), Expression::Identifier(object_name) if object_name == name)
            && matches!(property.as_ref(), Expression::String(property_name) if property_name == "push")
        {
            return Some(arguments);
        }
        None
    }

    fn simple_generator_array_rest_loop_body<'b>(
        statement: &'b Statement,
        name: &str,
    ) -> Option<(&'b Expression, &'b [Statement])> {
        let Statement::While {
            condition,
            body,
            break_hook: None,
            ..
        } = statement
        else {
            return None;
        };
        body.iter()
            .any(|effect| match effect {
                Statement::If { then_branch, .. } => then_branch.iter().any(|branch_effect| {
                    Self::simple_generator_array_push_arguments(branch_effect, name).is_some()
                }),
                effect => Self::simple_generator_array_push_arguments(effect, name).is_some(),
            })
            .then_some((condition, body.as_slice()))
    }

    fn simple_generator_apply_array_push_effect(
        &self,
        name: &str,
        effect: &Statement,
        prior_effects: &[Statement],
        elements: &mut Vec<ArrayElement>,
    ) -> bool {
        let Some(arguments) = Self::simple_generator_array_push_arguments(effect, name) else {
            return false;
        };
        for argument in arguments {
            let value = self
                .simple_generator_effect_expression(argument.expression(), prior_effects)
                .unwrap_or_else(|| self.materialize_static_expression(argument.expression()));
            elements.push(ArrayElement::Expression(value));
        }
        true
    }

    fn simple_generator_apply_array_rest_loop(
        &self,
        name: &str,
        condition: &Expression,
        body: &[Statement],
        prior_effects: &[Statement],
        elements: &mut Vec<ArrayElement>,
    ) -> Option<()> {
        let mut loop_effects = prior_effects.to_vec();
        for _ in 0..1024 {
            if !self.simple_generator_effect_condition_value(condition, &loop_effects)? {
                return Some(());
            }
            for effect in body {
                match effect {
                    Statement::If {
                        condition,
                        then_branch,
                        else_branch,
                    } => {
                        let branch = if self
                            .simple_generator_effect_condition_value(condition, &loop_effects)?
                        {
                            then_branch
                        } else {
                            else_branch
                        };
                        for branch_effect in branch {
                            self.simple_generator_apply_array_push_effect(
                                name,
                                branch_effect,
                                &loop_effects,
                                elements,
                            );
                        }
                    }
                    _ => {
                        self.simple_generator_apply_array_push_effect(
                            name,
                            effect,
                            &loop_effects,
                            elements,
                        );
                    }
                }
                loop_effects.push(effect.clone());
            }
        }
        None
    }

    fn simple_generator_static_getter_value(
        &self,
        source: &Expression,
        property: &Expression,
    ) -> Option<Expression> {
        let binding = self.resolve_member_getter_binding(source, property)?;
        let context = self.static_eval_context();
        let mut environment = self.snapshot_static_resolution_environment();
        execute_static_user_function_binding_in_environment(
            &context,
            &binding,
            &[],
            &mut environment,
            StaticFunctionEffectMode::Discard,
        )
    }

    fn simple_generator_copy_data_properties_binding(
        &self,
        source: &Expression,
        source_binding: &ObjectValueBinding,
        side_effects: &mut Vec<Expression>,
    ) -> Option<ObjectValueBinding> {
        let mut copied_binding = empty_object_value_binding();
        for name in ordered_object_property_names(source_binding) {
            if source_binding
                .non_enumerable_string_properties
                .iter()
                .any(|hidden_name| hidden_name == &name)
            {
                continue;
            }
            let property = Expression::String(name);
            let value = if self
                .resolve_member_getter_binding(source, &property)
                .is_some()
            {
                side_effects.push(Expression::Member {
                    object: Box::new(source.clone()),
                    property: Box::new(property.clone()),
                });
                self.simple_generator_static_getter_value(source, &property)?
            } else {
                object_binding_lookup_value(source_binding, &property)
                    .cloned()
                    .unwrap_or(Expression::Undefined)
            };
            object_binding_set_property(&mut copied_binding, property, value);
        }
        for (property, value) in &source_binding.symbol_properties {
            let value = if self
                .resolve_member_getter_binding(source, property)
                .is_some()
            {
                side_effects.push(Expression::Member {
                    object: Box::new(source.clone()),
                    property: Box::new(property.clone()),
                });
                self.simple_generator_static_getter_value(source, property)?
            } else {
                value.clone()
            };
            object_binding_set_property(&mut copied_binding, property.clone(), value);
        }
        Some(copied_binding)
    }

    fn simple_generator_effect_array_expression(
        &self,
        name: &str,
        effects: &[Statement],
    ) -> Option<Expression> {
        let mut elements = None;
        for (index, effect) in effects.iter().enumerate() {
            match effect {
                Statement::Let {
                    name: binding_name,
                    value: Expression::Array(value),
                    ..
                }
                | Statement::Var {
                    name: binding_name,
                    value: Expression::Array(value),
                }
                | Statement::Assign {
                    name: binding_name,
                    value: Expression::Array(value),
                } if binding_name == name => {
                    elements = Some(value.clone());
                }
                Statement::Let {
                    name: binding_name, ..
                }
                | Statement::Var {
                    name: binding_name, ..
                }
                | Statement::Assign {
                    name: binding_name, ..
                } if binding_name == name => {
                    elements = None;
                }
                effect @ Statement::Expression(Expression::Call { .. }) => {
                    let Some(elements) = elements.as_mut() else {
                        continue;
                    };
                    self.simple_generator_apply_array_push_effect(
                        name,
                        effect,
                        &effects[..index],
                        elements,
                    );
                }
                effect @ Statement::While { .. } => {
                    let Some(elements) = elements.as_mut() else {
                        continue;
                    };
                    let Some((condition, body)) =
                        Self::simple_generator_array_rest_loop_body(effect, name)
                    else {
                        continue;
                    };
                    self.simple_generator_apply_array_rest_loop(
                        name,
                        condition,
                        body,
                        &effects[..index],
                        elements,
                    )?;
                }
                _ => {}
            }
        }
        elements.map(Expression::Array)
    }

    fn simple_generator_member_from_static_object(
        &self,
        object: Expression,
        property: &Expression,
    ) -> Option<Expression> {
        if let Expression::Array(elements) = &object {
            if matches!(property, Expression::String(property_name) if property_name == "length") {
                return Some(Expression::Number(elements.len() as f64));
            }
            let index = match property {
                Expression::Number(index) if *index >= 0.0 && index.fract() == 0.0 => {
                    Some(*index as usize)
                }
                Expression::String(property_name) => property_name.parse::<usize>().ok(),
                _ => None,
            };
            return Some(
                index
                    .and_then(|index| elements.get(index))
                    .map(|element| match element {
                        ArrayElement::Expression(value) => value.clone(),
                        ArrayElement::Spread(_) => Expression::Undefined,
                    })
                    .unwrap_or(Expression::Undefined),
            );
        }
        let Expression::Object(entries) = object else {
            return None;
        };
        entries
            .into_iter()
            .find_map(|entry| {
                let ObjectEntry::Data { key, value } = entry else {
                    return None;
                };
                (self.materialize_static_expression(&key) == *property).then_some(value)
            })
            .or_else(|| {
                matches!(property, Expression::String(_) | Expression::Number(_))
                    .then_some(Expression::Undefined)
            })
    }

    fn expression_is_prior_iterator_step_binding(
        &self,
        expression: &Expression,
        effects: &[Statement],
    ) -> bool {
        matches!(
            expression,
            Expression::Identifier(name)
                if (name.starts_with("__ayy_array_step_")
                    || name.starts_with("__ayy_for_of_step_"))
                    && self
                        .simple_generator_effect_binding_value_with_index(name, effects)
                        .is_some()
        )
    }

    pub(in crate::backend::direct_wasm) fn simple_generator_effect_expression(
        &self,
        expression: &Expression,
        effects: &[Statement],
    ) -> Option<Expression> {
        match expression {
            Expression::Identifier(name) => {
                if let Some(value) = self.simple_generator_effect_array_expression(name, effects) {
                    return Some(value);
                }
                if let Some((index, value, prior_effects)) =
                    self.simple_generator_effect_binding_value_with_index(name, effects)
                {
                    let value = if Self::simple_generator_effect_call_is_iterator_next(value)
                        .is_some()
                    {
                        self.simple_generator_effect_iterator_step_object(name, value, effects)
                    } else {
                        self.simple_generator_effect_expression(value, prior_effects)
                            .or_else(|| {
                                let materialized = self.materialize_static_expression(value);
                                (!static_expression_matches(&materialized, value))
                                    .then_some(materialized)
                            })
                            .or_else(|| {
                                matches!(value, Expression::Identifier(_)).then(|| value.clone())
                            })
                    }?;
                    return Some(self.simple_generator_apply_binding_deletes(
                        name,
                        value,
                        &effects[index + 1..],
                    ));
                }
                self.resolve_function_binding_from_expression(expression)
                    .is_some()
                    .then(|| expression.clone())
            }
            Expression::Await(value) => self.simple_generator_effect_expression(value, effects),
            Expression::Member { object, property } => {
                let property = self.materialize_static_expression(property);
                if self.expression_is_prior_iterator_step_binding(object, effects) {
                    let object = self.simple_generator_effect_expression(object, effects)?;
                    return self.simple_generator_member_from_static_object(object, &property);
                }
                if let Some(value) =
                    self.simple_generator_current_iterator_step_member_value(object, &property)
                {
                    return Some(value);
                }
                let object = self.simple_generator_effect_expression(object, effects)?;
                self.simple_generator_member_from_static_object(object, &property)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                if self.simple_generator_effect_condition_value(condition, effects)? {
                    self.simple_generator_effect_expression(then_expression, effects)
                } else {
                    self.simple_generator_effect_expression(else_expression, effects)
                }
            }
            Expression::Binary { op, left, right } => {
                let left = self
                    .simple_generator_effect_expression(left, effects)
                    .unwrap_or_else(|| self.materialize_static_expression(left));
                let right = self
                    .simple_generator_effect_expression(right, effects)
                    .unwrap_or_else(|| self.materialize_static_expression(right));
                match (op, &left, &right) {
                    (BinaryOp::Add, Expression::Number(left), Expression::Number(right)) => {
                        Some(Expression::Number(left + right))
                    }
                    (BinaryOp::Subtract, Expression::Number(left), Expression::Number(right)) => {
                        Some(Expression::Number(left - right))
                    }
                    (BinaryOp::Multiply, Expression::Number(left), Expression::Number(right)) => {
                        Some(Expression::Number(left * right))
                    }
                    (BinaryOp::Divide, Expression::Number(left), Expression::Number(right)) => {
                        Some(Expression::Number(left / right))
                    }
                    (BinaryOp::Equal, _, _) => Some(Expression::Bool(left == right)),
                    (BinaryOp::NotEqual, _, _) => Some(Expression::Bool(left != right)),
                    (BinaryOp::LessThan, Expression::Number(left), Expression::Number(right)) => {
                        Some(Expression::Bool(left < right))
                    }
                    (
                        BinaryOp::LessThanOrEqual,
                        Expression::Number(left),
                        Expression::Number(right),
                    ) => Some(Expression::Bool(left <= right)),
                    (
                        BinaryOp::GreaterThan,
                        Expression::Number(left),
                        Expression::Number(right),
                    ) => Some(Expression::Bool(left > right)),
                    (
                        BinaryOp::GreaterThanOrEqual,
                        Expression::Number(left),
                        Expression::Number(right),
                    ) => Some(Expression::Bool(left >= right)),
                    _ => None,
                }
            }
            Expression::Array(elements) => Some(Expression::Array(
                elements
                    .iter()
                    .map(|element| match element {
                        ArrayElement::Expression(value) => Some(ArrayElement::Expression(
                            self.simple_generator_effect_expression(value, effects)
                                .unwrap_or_else(|| self.materialize_static_expression(value)),
                        )),
                        ArrayElement::Spread(_) => None,
                    })
                    .collect::<Option<Vec<_>>>()?,
            )),
            Expression::Object(entries) => {
                if entries
                    .iter()
                    .any(|entry| matches!(entry, ObjectEntry::Spread(_)))
                {
                    let mut object_binding = empty_object_value_binding();
                    let mut side_effects = Vec::new();
                    for entry in entries {
                        match entry {
                            ObjectEntry::Data { key, value } => {
                                let key = self.materialize_static_expression(key);
                                let value = self
                                    .simple_generator_effect_expression(value, effects)
                                    .unwrap_or_else(|| self.materialize_static_expression(value));
                                object_binding_set_property(&mut object_binding, key, value);
                            }
                            ObjectEntry::Spread(spread) => {
                                let spread = self
                                    .simple_generator_effect_expression(spread, effects)
                                    .unwrap_or_else(|| self.materialize_static_expression(spread));
                                if matches!(spread, Expression::Null | Expression::Undefined) {
                                    continue;
                                }
                                let spread_binding =
                                    self.simple_generator_spread_object_binding(&spread)?;
                                let copied_binding = self
                                    .simple_generator_copy_data_properties_binding(
                                        &spread,
                                        &spread_binding,
                                        &mut side_effects,
                                    )?;
                                merge_enumerable_object_binding(
                                    &mut object_binding,
                                    &copied_binding,
                                );
                            }
                            ObjectEntry::Getter { .. } | ObjectEntry::Setter { .. } => {
                                return None;
                            }
                        }
                    }
                    let object_expression = object_binding_to_expression(&object_binding);
                    if side_effects.is_empty() {
                        Some(object_expression)
                    } else {
                        side_effects.push(object_expression);
                        Some(Expression::Sequence(side_effects))
                    }
                } else {
                    Some(Expression::Object(
                        entries
                            .iter()
                            .map(|entry| match entry {
                                ObjectEntry::Data { key, value } => Some(ObjectEntry::Data {
                                    key: self.materialize_static_expression(key),
                                    value: self
                                        .simple_generator_effect_expression(value, effects)
                                        .unwrap_or_else(|| {
                                            self.materialize_static_expression(value)
                                        }),
                                }),
                                _ => None,
                            })
                            .collect::<Option<Vec<_>>>()?,
                    ))
                }
            }
            _ => Some(self.materialize_static_expression(expression)),
        }
    }

    fn simple_generator_current_iterator_step_member_value(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<Expression> {
        let IteratorStepBinding::Runtime {
            static_done,
            static_value,
            value_candidates,
            ..
        } = self.resolve_iterator_step_binding_from_expression(object)?;
        match property {
            Expression::String(property_name) if property_name == "done" => {
                static_done.map(Expression::Bool)
            }
            Expression::String(property_name) if property_name == "value" => static_value
                .map(|value| self.materialize_static_expression(&value))
                .or_else(|| {
                    let [candidate] = value_candidates.as_slice() else {
                        return None;
                    };
                    Some(self.materialize_static_expression(candidate))
                }),
            _ => None,
        }
    }

    fn simple_generator_effect_condition_value(
        &self,
        condition: &Expression,
        effects: &[Statement],
    ) -> Option<bool> {
        match condition {
            Expression::Binary { op, left, right } => {
                let left = self.simple_generator_effect_expression(left, effects)?;
                let right = self.simple_generator_effect_expression(right, effects)?;
                match op {
                    BinaryOp::Equal => Some(left == right),
                    BinaryOp::NotEqual => Some(left != right),
                    BinaryOp::LogicalOr => Some(
                        self.simple_generator_effect_condition_value(&left, effects)
                            .unwrap_or(false)
                            || self
                                .simple_generator_effect_condition_value(&right, effects)
                                .unwrap_or(false),
                    ),
                    BinaryOp::LogicalAnd => Some(
                        self.simple_generator_effect_condition_value(&left, effects)
                            .unwrap_or(false)
                            && self
                                .simple_generator_effect_condition_value(&right, effects)
                                .unwrap_or(false),
                    ),
                    _ => None,
                }
            }
            Expression::Bool(value) => Some(*value),
            _ => self
                .simple_generator_effect_expression(condition, effects)
                .and_then(|value| self.resolve_static_if_condition_value(&value))
                .or_else(|| {
                    let materialized = self.materialize_static_expression(condition);
                    self.resolve_static_if_condition_value(&materialized)
                }),
        }
    }

    pub(in crate::backend::direct_wasm) fn analyze_simple_generator_statements(
        &self,
        statements: &[Statement],
        async_generator: bool,
        steps: &mut Vec<SimpleGeneratorStep>,
        effects: &mut Vec<Statement>,
    ) -> Option<()> {
        self.analyze_simple_generator_statements_with_close_effects(
            statements,
            async_generator,
            steps,
            effects,
            &[],
        )
    }

    fn lowered_generator_finally_flags(catch_body: &[Statement]) -> Option<(String, String)> {
        let [
            Statement::Assign {
                name: threw_name,
                value: Expression::Bool(true),
            },
            Statement::Assign {
                name: error_name,
                value: Expression::Identifier(_),
            },
        ] = catch_body
        else {
            return None;
        };
        Some((threw_name.clone(), error_name.clone()))
    }

    fn lowered_generator_finally_rethrow_index(
        statements: &[Statement],
        start: usize,
        threw_name: &str,
        error_name: &str,
    ) -> Option<usize> {
        statements
            .iter()
            .enumerate()
            .skip(start)
            .find_map(|(index, statement)| {
                let Statement::If {
                    condition: Expression::Identifier(condition_name),
                    then_branch,
                    else_branch,
                } = statement
                else {
                    return None;
                };
                if condition_name != threw_name || !else_branch.is_empty() {
                    return None;
                }
                let [Statement::Throw(Expression::Identifier(thrown_name))] =
                    then_branch.as_slice()
                else {
                    return None;
                };
                (thrown_name == error_name).then_some(index)
            })
    }

    fn lowered_for_of_step_pattern<'b>(
        iterator_name: &str,
        body: &'b [Statement],
    ) -> Option<(&'b str, &'b Statement, &'b [Statement])> {
        let [
            Statement::Let {
                name: step_name,
                value: Expression::Call { callee, arguments },
                ..
            },
            Statement::If { condition, .. },
            value_binding @ Statement::Let {
                value:
                    Expression::Member {
                        object: value_object,
                        property: value_property,
                    },
                ..
            },
            rest @ ..,
        ] = body
        else {
            return None;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return None;
        };
        if !arguments.is_empty()
            || !matches!(object.as_ref(), Expression::Identifier(name) if name == iterator_name)
            || !matches!(property.as_ref(), Expression::String(name) if name == "next")
        {
            return None;
        }
        if !matches!(
            condition,
            Expression::Member { object, property }
                if matches!(object.as_ref(), Expression::Identifier(name) if name == step_name)
                    && matches!(property.as_ref(), Expression::String(name) if name == "done")
        ) {
            return None;
        }
        if !matches!(value_object.as_ref(), Expression::Identifier(name) if name == step_name)
            || !matches!(value_property.as_ref(), Expression::String(name) if name == "value")
        {
            return None;
        }
        Some((step_name.as_str(), value_binding, rest))
    }

    fn expand_static_lowered_for_of_for_simple_generator(
        &self,
        statement: &Statement,
        prior_effects: &[Statement],
    ) -> Option<Vec<Statement>> {
        let Statement::For {
            init,
            condition: Some(Expression::Bool(true)),
            update: None,
            body,
            ..
        } = statement
        else {
            return None;
        };
        let (iterator_name, _) = init.iter().find_map(|statement| {
            let Statement::Let {
                name,
                value: Expression::GetIterator(source),
                ..
            } = statement
            else {
                return None;
            };
            Some((name.as_str(), source.as_ref()))
        })?;
        let (step_name, value_binding, iteration_body) =
            Self::lowered_for_of_step_pattern(iterator_name, body)?;
        let mut iterator_effects = prior_effects.to_vec();
        iterator_effects.extend(init.iter().cloned());
        let values =
            self.simple_generator_effect_iterator_source_values(iterator_name, &iterator_effects)?;

        let mut expanded = init.clone();
        for value in values {
            let mut block_body = vec![Statement::Let {
                name: step_name.to_string(),
                mutable: true,
                value: Expression::Object(vec![
                    ObjectEntry::Data {
                        key: Expression::String("done".to_string()),
                        value: Expression::Bool(false),
                    },
                    ObjectEntry::Data {
                        key: Expression::String("value".to_string()),
                        value: value.unwrap_or(Expression::Undefined),
                    },
                ]),
            }];
            block_body.extend(iteration_body.iter().cloned());
            block_body.insert(1, value_binding.clone());
            expanded.push(Statement::Block { body: block_body });
        }
        Some(expanded)
    }

    fn expand_static_throwing_try_catch_for_simple_generator(
        body: &[Statement],
        catch_binding: &Option<String>,
        catch_setup: &[Statement],
        catch_body: &[Statement],
    ) -> Option<Vec<Statement>> {
        let throw_index = body.iter().position(|statement| {
            matches!(statement, Statement::Throw(_))
                && !Self::statement_contains_generator_yield(statement)
        })?;
        if body[..throw_index]
            .iter()
            .any(Self::statement_contains_generator_yield)
        {
            return None;
        }
        let Statement::Throw(value) = &body[throw_index] else {
            return None;
        };
        let mut expanded = body[..throw_index].to_vec();
        if let Some(catch_binding) = catch_binding {
            expanded.push(Statement::Let {
                name: catch_binding.clone(),
                mutable: true,
                value: value.clone(),
            });
        }
        expanded.extend(catch_setup.iter().cloned());
        expanded.extend(catch_body.iter().cloned());
        Some(expanded)
    }

    fn analyze_simple_generator_statements_with_close_effects(
        &self,
        statements: &[Statement],
        async_generator: bool,
        steps: &mut Vec<SimpleGeneratorStep>,
        effects: &mut Vec<Statement>,
        active_close_effects: &[Statement],
    ) -> Option<()> {
        let trace_analyze = std::env::var_os("AYY_TRACE_SIMPLE_GENERATOR_SOURCE").is_some();
        for (index, statement) in statements.iter().enumerate() {
            if trace_analyze {
                eprintln!("simple_generator_analyze:statement={statement:?}");
            }
            match statement {
                Statement::Yield { value } => {
                    steps.push(SimpleGeneratorStep {
                        effects: std::mem::take(effects),
                        close_effects: active_close_effects.to_vec(),
                        outcome: SimpleGeneratorStepOutcome::Yield(value.clone()),
                    });
                }
                Statement::YieldDelegate { value } => {
                    let (mut delegate_steps, mut delegate_completion_effects) =
                        self.resolve_simple_yield_delegate_source(value, async_generator)?;
                    let delegate_ends_in_throw = delegate_steps.last().is_some_and(|step| {
                        matches!(step.outcome, SimpleGeneratorStepOutcome::Throw(_))
                    });
                    if let Some(first_step) = delegate_steps.first_mut() {
                        let mut prefix_effects = std::mem::take(effects);
                        prefix_effects.append(&mut first_step.effects);
                        first_step.effects = prefix_effects;
                    }
                    if !active_close_effects.is_empty() {
                        for step in &mut delegate_steps {
                            step.close_effects
                                .extend(active_close_effects.iter().cloned());
                        }
                    }
                    steps.extend(delegate_steps);
                    effects.append(&mut delegate_completion_effects);
                    if delegate_ends_in_throw {
                        return Some(());
                    }
                }
                Statement::Throw(value) => {
                    steps.push(SimpleGeneratorStep {
                        effects: std::mem::take(effects),
                        close_effects: Vec::new(),
                        outcome: SimpleGeneratorStepOutcome::Throw(value.clone()),
                    });
                    return Some(());
                }
                Statement::Block { body } => {
                    self.analyze_simple_generator_statements_with_close_effects(
                        body,
                        async_generator,
                        steps,
                        effects,
                        active_close_effects,
                    )?;
                }
                Statement::Try {
                    body,
                    catch_binding,
                    catch_setup,
                    catch_body,
                } if catch_setup
                    .iter()
                    .chain(catch_body)
                    .any(Self::statement_contains_generator_yield) =>
                {
                    let expanded = Self::expand_static_throwing_try_catch_for_simple_generator(
                        body,
                        catch_binding,
                        catch_setup,
                        catch_body,
                    )?;
                    self.analyze_simple_generator_statements_with_close_effects(
                        &expanded,
                        async_generator,
                        steps,
                        effects,
                        active_close_effects,
                    )?;
                }
                Statement::Try {
                    body,
                    catch_setup,
                    catch_body,
                    ..
                } if !catch_setup
                    .iter()
                    .any(Self::statement_contains_generator_yield)
                    && !catch_body
                        .iter()
                        .any(Self::statement_contains_generator_yield) =>
                {
                    let scoped_close_effects = if let Some((threw_name, error_name)) =
                        Self::lowered_generator_finally_flags(catch_body)
                        && let Some(rethrow_index) = Self::lowered_generator_finally_rethrow_index(
                            statements,
                            index + 1,
                            &threw_name,
                            &error_name,
                        ) {
                        let mut close_effects = statements[index + 1..=rethrow_index].to_vec();
                        close_effects.extend(active_close_effects.iter().cloned());
                        close_effects
                    } else {
                        active_close_effects.to_vec()
                    };
                    self.analyze_simple_generator_statements_with_close_effects(
                        body,
                        async_generator,
                        steps,
                        effects,
                        &scoped_close_effects,
                    )?;
                }
                Statement::Var { .. } | Statement::Let { .. } => {
                    effects.push(statement.clone());
                }
                Statement::If {
                    condition,
                    then_branch,
                    else_branch,
                } => {
                    if let Some(condition_value) =
                        self.simple_generator_effect_condition_value(condition, effects)
                    {
                        let branch = if condition_value {
                            then_branch
                        } else {
                            else_branch
                        };
                        self.analyze_simple_generator_statements_with_close_effects(
                            branch,
                            async_generator,
                            steps,
                            effects,
                            active_close_effects,
                        )?;
                    } else if else_branch.is_empty()
                        && !then_branch
                            .iter()
                            .any(Self::statement_contains_generator_yield)
                    {
                        effects.push(statement.clone());
                    } else {
                        if then_branch
                            .iter()
                            .any(Self::statement_contains_generator_yield)
                            || else_branch
                                .iter()
                                .any(Self::statement_contains_generator_yield)
                        {
                            if trace_analyze {
                                eprintln!(
                                    "simple_generator_analyze:reject-unresolved-yielding-if condition={condition:?}"
                                );
                            }
                            return None;
                        }
                        effects.push(statement.clone());
                    }
                }
                Statement::Assign { .. }
                | Statement::AssignMember { .. }
                | Statement::Expression(_)
                | Statement::Print { .. } => effects.push(statement.clone()),
                Statement::For { .. } if Self::statement_contains_generator_yield(statement) => {
                    let expanded =
                        self.expand_static_lowered_for_of_for_simple_generator(statement, effects)?;
                    self.analyze_simple_generator_statements_with_close_effects(
                        &expanded,
                        async_generator,
                        steps,
                        effects,
                        active_close_effects,
                    )?;
                }
                Statement::Declaration { .. }
                | Statement::Labeled { .. }
                | Statement::With { .. }
                | Statement::Try { .. }
                | Statement::Switch { .. }
                | Statement::For { .. }
                | Statement::While { .. }
                | Statement::DoWhile { .. }
                    if !Self::statement_contains_generator_yield(statement) =>
                {
                    effects.push(statement.clone());
                }
                _ => {
                    if trace_analyze {
                        eprintln!("simple_generator_analyze:reject statement={statement:?}");
                    }
                    return None;
                }
            }
        }

        Some(())
    }

    pub(in crate::backend::direct_wasm) fn simple_generator_call_arguments(
        &self,
        call_argument_values: &[Expression],
    ) -> Vec<CallArgument> {
        call_argument_values
            .iter()
            .cloned()
            .map(CallArgument::Expression)
            .collect()
    }

    pub(in crate::backend::direct_wasm) fn simple_generator_arguments_binding_expression(
        &self,
        arguments_values: &[Expression],
    ) -> Expression {
        Expression::Array(
            arguments_values
                .iter()
                .cloned()
                .map(crate::ir::hir::ArrayElement::Expression)
                .collect(),
        )
    }

    pub(in crate::backend::direct_wasm) fn simple_generator_arguments_are_shadowed(
        &self,
        user_function: &UserFunction,
    ) -> bool {
        user_function.body_declares_arguments_binding
            || user_function
                .params
                .iter()
                .any(|param| param == "arguments")
    }

    pub(in crate::backend::direct_wasm) fn update_simple_generator_call_frame_state(
        &self,
        original_statement: &Statement,
        transformed_statement: &Statement,
        user_function: &UserFunction,
        mapped_arguments: bool,
        call_argument_values: &mut Vec<Expression>,
        arguments_values: &mut Vec<Expression>,
    ) {
        if let Statement::Assign { name, value } = transformed_statement
            && let Some(index) = user_function.params.iter().position(|param| param == name)
        {
            if index >= call_argument_values.len() {
                call_argument_values.resize(index + 1, Expression::Undefined);
            }
            call_argument_values[index] = value.clone();
            if mapped_arguments && index < arguments_values.len() {
                arguments_values[index] = value.clone();
            }
            return;
        }

        let Statement::AssignMember {
            object: original_object,
            ..
        } = original_statement
        else {
            return;
        };
        if self.simple_generator_arguments_are_shadowed(user_function)
            || !matches!(original_object, Expression::Identifier(name) if name == "arguments")
        {
            return;
        }
        let Statement::AssignMember {
            property, value, ..
        } = transformed_statement
        else {
            return;
        };
        let Some(index) = argument_index_from_expression(property).map(|index| index as usize)
        else {
            return;
        };
        if index >= arguments_values.len() {
            arguments_values.resize(index + 1, Expression::Undefined);
        }
        arguments_values[index] = value.clone();
        if mapped_arguments
            && index < user_function.params.len()
            && index < call_argument_values.len()
        {
            call_argument_values[index] = value.clone();
        }
    }
}
