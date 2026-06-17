use super::*;

fn context_expression_references_internal_iterator_step(expression: &Expression) -> bool {
    match expression {
        Expression::Identifier(name) => {
            name.starts_with("__ayy_array_step_")
                || name.starts_with("__ayy_for_of_step_")
                || name.starts_with("__ayy_array_iter_value_")
                || name.starts_with("__ayy_for_of_iter_value_")
                || name.starts_with("__ayy_binding_value_")
        }
        Expression::Array(elements) => elements.iter().any(|element| match element {
            ArrayElement::Expression(value) | ArrayElement::Spread(value) => {
                context_expression_references_internal_iterator_step(value)
            }
        }),
        Expression::Object(entries) => entries.iter().any(|entry| match entry {
            ObjectEntry::Data { key, value } => {
                context_expression_references_internal_iterator_step(key)
                    || context_expression_references_internal_iterator_step(value)
            }
            ObjectEntry::Getter { key, getter } => {
                context_expression_references_internal_iterator_step(key)
                    || context_expression_references_internal_iterator_step(getter)
            }
            ObjectEntry::Setter { key, setter } => {
                context_expression_references_internal_iterator_step(key)
                    || context_expression_references_internal_iterator_step(setter)
            }
            ObjectEntry::Spread(value) => {
                context_expression_references_internal_iterator_step(value)
            }
        }),
        Expression::Binary { left, right, .. } => {
            context_expression_references_internal_iterator_step(left)
                || context_expression_references_internal_iterator_step(right)
        }
        Expression::Conditional {
            condition,
            then_expression,
            else_expression,
        } => {
            context_expression_references_internal_iterator_step(condition)
                || context_expression_references_internal_iterator_step(then_expression)
                || context_expression_references_internal_iterator_step(else_expression)
        }
        Expression::Member { object, property } => {
            context_expression_references_internal_iterator_step(object)
                || context_expression_references_internal_iterator_step(property)
        }
        Expression::Unary { expression, .. }
        | Expression::Await(expression)
        | Expression::EnumerateKeys(expression)
        | Expression::GetIterator(expression)
        | Expression::IteratorClose(expression) => {
            context_expression_references_internal_iterator_step(expression)
        }
        Expression::Assign { value, .. } => {
            context_expression_references_internal_iterator_step(value)
        }
        Expression::AssignMember {
            object,
            property,
            value,
        } => {
            context_expression_references_internal_iterator_step(object)
                || context_expression_references_internal_iterator_step(property)
                || context_expression_references_internal_iterator_step(value)
        }
        Expression::AssignSuperMember { property, value } => {
            context_expression_references_internal_iterator_step(property)
                || context_expression_references_internal_iterator_step(value)
        }
        Expression::Call { callee, arguments }
        | Expression::New { callee, arguments }
        | Expression::SuperCall { callee, arguments } => {
            context_expression_references_internal_iterator_step(callee)
                || arguments.iter().any(|argument| match argument {
                    CallArgument::Expression(value) | CallArgument::Spread(value) => {
                        context_expression_references_internal_iterator_step(value)
                    }
                })
        }
        Expression::SuperMember { property } => {
            context_expression_references_internal_iterator_step(property)
        }
        Expression::Sequence(expressions) => expressions
            .iter()
            .any(context_expression_references_internal_iterator_step),
        _ => false,
    }
}

fn expression_is_dynamic_import_call(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::Call { callee, .. }
            if matches!(callee.as_ref(), Expression::Identifier(name) if name == "__ayyDynamicImport")
    )
}

fn expression_is_promise_all_call(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::Call { callee, .. }
            if matches!(
                callee.as_ref(),
                Expression::Member { object, property }
                    if matches!(object.as_ref(), Expression::Identifier(name) if name == "Promise")
                        && matches!(property.as_ref(), Expression::String(name) if name == "all")
            )
    )
}

fn expression_is_static_promise_source_call(expression: &Expression) -> bool {
    let Expression::Call { callee, .. } = expression else {
        return false;
    };
    let Expression::Member { object, property } = callee.as_ref() else {
        return false;
    };
    matches!(object.as_ref(), Expression::Identifier(name) if name == "Promise")
        && matches!(
            property.as_ref(),
            Expression::String(name)
                if matches!(
                    name.as_str(),
                    "resolve" | "reject" | "all" | "allSettled" | "any" | "race"
                )
        )
}

fn expression_is_static_promise_then_call(expression: &Expression) -> bool {
    let Expression::Call { callee, .. } = expression else {
        return false;
    };
    let Expression::Member { object, property } = callee.as_ref() else {
        return false;
    };
    matches!(property.as_ref(), Expression::String(name) if name == "then")
        && (expression_is_static_promise_source_call(object)
            || expression_is_static_promise_then_call(object))
}

pub(in crate::backend::direct_wasm) fn expression_is_dynamic_module_namespace_descriptor_call(
    compiler: &FunctionCompiler<'_>,
    expression: &Expression,
) -> bool {
    let Expression::Call { callee, arguments } = expression else {
        return false;
    };
    if !matches!(
        callee.as_ref(),
        Expression::Member { object, property }
            if matches!(object.as_ref(), Expression::Identifier(name) if name == "Object" || name == "Reflect")
                && matches!(
                    property.as_ref(),
                    Expression::String(name) if name == "getOwnPropertyDescriptor"
                )
    ) {
        return false;
    }
    let [
        CallArgument::Expression(target),
        CallArgument::Expression(property),
        ..,
    ] = arguments.as_slice()
    else {
        return false;
    };
    if compiler
        .module_namespace_index_from_expression(target)
        .is_none()
    {
        return false;
    }
    let materialized_property = compiler
        .resolve_property_key_expression(property)
        .unwrap_or_else(|| compiler.materialize_static_expression(property));
    static_property_name_from_expression(&materialized_property).is_none()
        && !is_symbol_to_string_tag_expression(&materialized_property)
}

fn promise_resolve_array_placeholder() -> Expression {
    Expression::Call {
        callee: Box::new(Expression::Member {
            object: Box::new(Expression::Identifier("Promise".to_string())),
            property: Box::new(Expression::String("resolve".to_string())),
        }),
        arguments: vec![CallArgument::Expression(Expression::Array(Vec::new()))],
    }
}

fn expression_is_promise_resolve_undefined_call(expression: &Expression) -> bool {
    let Expression::Call { callee, arguments } = expression else {
        return false;
    };
    let Expression::Member { object, property } = callee.as_ref() else {
        return false;
    };
    matches!(object.as_ref(), Expression::Identifier(name) if name == "Promise")
        && matches!(property.as_ref(), Expression::String(name) if name == "resolve")
        && matches!(
            arguments.as_slice(),
            [] | [CallArgument::Expression(Expression::Undefined)]
        )
}

fn expression_is_static_promise_with_resolvers_record(expression: &Expression) -> bool {
    let Expression::Object(entries) = expression else {
        return false;
    };
    let mut has_promise = false;
    let mut has_resolve = false;
    let mut has_reject = false;
    for entry in entries {
        let ObjectEntry::Data {
            key: Expression::String(key),
            value,
        } = entry
        else {
            continue;
        };
        match key.as_str() {
            "promise" => has_promise = expression_is_promise_resolve_undefined_call(value),
            "resolve" => {
                has_resolve = matches!(
                    value,
                    Expression::Identifier(name)
                        if name == "__ayy_promise_with_resolvers_resolve"
                );
            }
            "reject" => {
                has_reject = matches!(
                    value,
                    Expression::Identifier(name)
                        if name == "__ayy_promise_with_resolvers_reject"
                );
            }
            _ => {}
        }
    }
    has_promise && has_resolve && has_reject
}

fn expression_is_array_length_member(expression: &Expression) -> Option<&Expression> {
    let Expression::Member { object, property } = expression else {
        return None;
    };
    matches!(property.as_ref(), Expression::String(name) if name == "length")
        .then_some(object.as_ref())
}

fn async_delegate_result_member_kind(expression: &Expression) -> Option<StaticValueKind> {
    let Expression::Member { object, property } = expression else {
        return None;
    };
    if !matches!(object.as_ref(), Expression::Identifier(name) if name.starts_with("__ayy_async_delegate_result_"))
    {
        return None;
    }
    let Expression::String(property_name) = property.as_ref() else {
        return None;
    };
    match property_name.as_str() {
        "done" => Some(StaticValueKind::Bool),
        "value" => Some(StaticValueKind::Unknown),
        _ => None,
    }
}

fn expression_is_non_prototype_nested_member(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::Member { object, .. }
            if matches!(
                object.as_ref(),
                Expression::Member { property, .. }
                    if !matches!(property.as_ref(), Expression::String(name) if name == "prototype")
            )
    )
}

fn expression_is_nested_assert_helper_member(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::Member { object, .. }
            if matches!(
                object.as_ref(),
                Expression::Member { object: root, .. }
                    if matches!(root.as_ref(), Expression::Identifier(name) if name == "assert")
            )
    )
}

fn expression_is_nested_assert_helper_runtime_value(expression: &Expression) -> bool {
    expression_is_nested_assert_helper_member(expression)
        || matches!(
            expression,
            Expression::Call { callee, .. }
                if expression_is_nested_assert_helper_member(callee)
        )
}

fn static_promise_with_resolvers_object_binding() -> ObjectValueBinding {
    let mut binding = empty_object_value_binding();
    binding.string_properties.push((
        "promise".to_string(),
        Expression::Call {
            callee: Box::new(Expression::Member {
                object: Box::new(Expression::Identifier("Promise".to_string())),
                property: Box::new(Expression::String("resolve".to_string())),
            }),
            arguments: vec![CallArgument::Expression(Expression::Undefined)],
        },
    ));
    binding.string_properties.push((
        "resolve".to_string(),
        Expression::Identifier("__ayy_promise_with_resolvers_resolve".to_string()),
    ));
    binding.string_properties.push((
        "reject".to_string(),
        Expression::Identifier("__ayy_promise_with_resolvers_reject".to_string()),
    ));
    binding
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum NullishAssignmentCheck {
    Undefined,
    Null,
}

impl<'a> FunctionCompiler<'a> {
    fn array_literal_store_prototype_source() -> Expression {
        Expression::New {
            callee: Box::new(Expression::Identifier("Array".to_string())),
            arguments: Vec::new(),
        }
    }

    fn return_object_entries(statement: &Statement) -> Option<&[ObjectEntry]> {
        let Statement::Return(Expression::Object(entries)) = statement else {
            return None;
        };
        Some(entries)
    }

    fn single_statement_return_object_entries(statements: &[Statement]) -> Option<&[ObjectEntry]> {
        let [statement] = statements else {
            return None;
        };
        Self::return_object_entries(statement)
    }

    fn object_entry_data_value<'b>(
        entries: &'b [ObjectEntry],
        property_name: &str,
    ) -> Option<&'b Expression> {
        entries.iter().find_map(|entry| {
            let ObjectEntry::Data {
                key: Expression::String(key),
                value,
            } = entry
            else {
                return None;
            };
            (key == property_name).then_some(value)
        })
    }

    fn expression_is_symbol_iterator_key(expression: &Expression) -> bool {
        match expression {
            Expression::Sequence(expressions) => {
                matches!(expressions.as_slice(), [expression] if Self::expression_is_symbol_iterator_key(expression))
            }
            Expression::Member { object, property } => {
                matches!(object.as_ref(), Expression::Identifier(name) if name == "Symbol")
                    && matches!(property.as_ref(), Expression::String(name) if name == "iterator")
            }
            _ => false,
        }
    }

    fn object_entry_symbol_iterator_value(entries: &[ObjectEntry]) -> Option<&Expression> {
        entries.iter().find_map(|entry| {
            let ObjectEntry::Data { key, value } = entry else {
                return None;
            };
            Self::expression_is_symbol_iterator_key(key).then_some(value)
        })
    }

    fn identifier_name(expression: &Expression) -> Option<&str> {
        let Expression::Identifier(name) = expression else {
            return None;
        };
        Some(name)
    }

    fn binary_identifier_addend<'b>(
        expression: &'b Expression,
        known_name: &str,
    ) -> Option<&'b str> {
        let Expression::Binary {
            op: BinaryOp::Add,
            left,
            right,
        } = expression
        else {
            return None;
        };
        match (Self::identifier_name(left), Self::identifier_name(right)) {
            (Some(left), Some(right)) if left == known_name => Some(right),
            (Some(left), Some(right)) if right == known_name => Some(left),
            _ => None,
        }
    }

    fn expression_is_identifier_plus_one(expression: &Expression, identifier: &str) -> bool {
        let Expression::Binary {
            op: BinaryOp::Add,
            left,
            right,
        } = expression
        else {
            return false;
        };
        (matches!(left.as_ref(), Expression::Identifier(name) if name == identifier)
            && matches!(right.as_ref(), Expression::Number(value) if *value == 1.0))
            || (matches!(right.as_ref(), Expression::Identifier(name) if name == identifier)
                && matches!(left.as_ref(), Expression::Number(value) if *value == 1.0))
    }

    fn expression_is_identifier_pair_add(
        expression: &Expression,
        left_name: &str,
        right_name: &str,
    ) -> bool {
        let Expression::Binary {
            op: BinaryOp::Add,
            left,
            right,
        } = expression
        else {
            return false;
        };
        matches!(
            (Self::identifier_name(left), Self::identifier_name(right)),
            (Some(left), Some(right))
                if (left == left_name && right == right_name)
                    || (left == right_name && right == left_name)
        )
    }

    pub(in crate::backend::direct_wasm) fn static_counted_iterator_factory_call_binding(
        &self,
        expression: &Expression,
    ) -> Option<ArrayValueBinding> {
        let Expression::Call { callee, arguments } = expression else {
            return None;
        };
        if arguments
            .iter()
            .any(|argument| matches!(argument, CallArgument::Spread(_)))
        {
            return None;
        }
        let LocalFunctionBinding::User(factory_name) =
            self.resolve_function_binding_from_expression(callee)?
        else {
            return None;
        };
        let factory_function = self.resolve_registered_function_declaration(&factory_name)?;
        if factory_function.kind != FunctionKind::Ordinary
            || factory_function
                .params
                .iter()
                .any(|param| param.default.is_some() || param.rest || param.name == "arguments")
        {
            return None;
        }
        let factory_return_entries =
            Self::single_statement_return_object_entries(&factory_function.body)?;
        let iterator_function_name = Self::identifier_name(
            Self::object_entry_symbol_iterator_value(factory_return_entries)?,
        )?
        .to_string();

        let iterator_function =
            self.resolve_registered_function_declaration(&iterator_function_name)?;
        let [
            Statement::Var {
                name: index_name,
                value: Expression::Number(index_start),
            },
            iterator_return,
        ] = iterator_function.body.as_slice()
        else {
            return None;
        };
        if *index_start != 0.0 {
            return None;
        }
        let iterator_return_entries = Self::return_object_entries(iterator_return)?;
        let next_function_name = Self::identifier_name(Self::object_entry_data_value(
            iterator_return_entries,
            "next",
        )?)?
        .to_string();

        let next_function = self.resolve_registered_function_declaration(&next_function_name)?;
        let [step_if, completion_return] = next_function.body.as_slice() else {
            return None;
        };
        let Statement::If {
            condition,
            then_branch,
            else_branch,
        } = step_if
        else {
            return None;
        };
        if !else_branch.is_empty() {
            return None;
        }
        let Expression::Binary {
            op: BinaryOp::LessThan,
            left,
            right,
        } = condition
        else {
            return None;
        };
        if !matches!(left.as_ref(), Expression::Identifier(name) if name == index_name) {
            return None;
        }
        let count_name = Self::identifier_name(right)?;

        let then_statements = match then_branch.as_slice() {
            [Statement::Block { body }] => body.as_slice(),
            statements => statements,
        };
        let [
            Statement::Var {
                name: value_name,
                value: step_value_expression,
            },
            Statement::Assign {
                name: assigned_index_name,
                value: index_update_expression,
            },
            step_return,
        ] = then_statements
        else {
            return None;
        };
        if assigned_index_name != index_name
            || !Self::expression_is_identifier_plus_one(index_update_expression, index_name)
        {
            return None;
        }
        let base_name = Self::binary_identifier_addend(step_value_expression, index_name)?;

        let step_return_entries = Self::return_object_entries(step_return)?;
        if !matches!(
            Self::object_entry_data_value(step_return_entries, "value"),
            Some(Expression::Identifier(name)) if name == value_name
        ) || !matches!(
            Self::object_entry_data_value(step_return_entries, "done"),
            Some(Expression::Bool(false))
        ) {
            return None;
        }

        let completion_return_entries = Self::return_object_entries(completion_return)?;
        if !matches!(
            Self::object_entry_data_value(completion_return_entries, "value"),
            Some(expression) if Self::expression_is_identifier_pair_add(expression, base_name, count_name)
        ) || !matches!(
            Self::object_entry_data_value(completion_return_entries, "done"),
            Some(Expression::Bool(true))
        ) {
            return None;
        }

        let base_param_index = factory_function
            .params
            .iter()
            .position(|param| param.name == base_name)?;
        let count_param_index = factory_function
            .params
            .iter()
            .position(|param| param.name == count_name)?;
        let base_argument = match arguments.get(base_param_index)? {
            CallArgument::Expression(expression) => expression,
            CallArgument::Spread(_) => return None,
        };
        let count_argument = match arguments.get(count_param_index)? {
            CallArgument::Expression(expression) => expression,
            CallArgument::Spread(_) => return None,
        };
        let base = self.resolve_static_number_value(base_argument)?;
        let count = self.resolve_static_number_value(count_argument)?;
        if !base.is_finite()
            || !count.is_finite()
            || count < 0.0
            || count.fract() != 0.0
            || count > 256.0
        {
            return None;
        }
        Some(ArrayValueBinding {
            values: (0..count as usize)
                .map(|index| Some(Expression::Number(base + index as f64)))
                .collect(),
        })
    }

    pub(in crate::backend::direct_wasm) fn static_array_literal_spread_binding(
        &self,
        expression: &Expression,
    ) -> Option<ArrayValueBinding> {
        self.array_literal_spread_binding(expression, false)
    }

    fn array_literal_spread_binding(
        &self,
        expression: &Expression,
        allow_runtime_reads: bool,
    ) -> Option<ArrayValueBinding> {
        self.array_literal_store_binding(expression, allow_runtime_reads)
            .or_else(|| self.static_counted_iterator_factory_call_binding(expression))
            .or_else(|| {
                matches!(expression, Expression::Identifier(_))
                    .then(|| self.resolve_array_binding_from_expression(expression))
                    .flatten()
            })
    }

    pub(in crate::backend::direct_wasm) fn static_array_literal_store_binding(
        &self,
        expression: &Expression,
    ) -> Option<ArrayValueBinding> {
        self.array_literal_store_binding(expression, false)
    }

    fn runtime_array_literal_store_binding(
        &self,
        expression: &Expression,
    ) -> Option<ArrayValueBinding> {
        self.array_literal_store_binding(expression, true)
    }

    fn array_literal_store_binding(
        &self,
        expression: &Expression,
        allow_runtime_reads: bool,
    ) -> Option<ArrayValueBinding> {
        let Expression::Array(elements) = expression else {
            return None;
        };
        let mut values = Vec::new();
        for element in elements {
            match element {
                ArrayElement::Expression(expression) => {
                    if crate::ir::hir::expression_is_array_elision(expression) {
                        values.push(None);
                    } else {
                        values.push(Some(
                            self.array_literal_store_value(expression, allow_runtime_reads)?,
                        ));
                    }
                }
                ArrayElement::Spread(expression) => {
                    let spread_binding =
                        self.array_literal_spread_binding(expression, allow_runtime_reads)?;
                    values.extend(
                        spread_binding
                            .values
                            .into_iter()
                            .map(|value| Some(value.unwrap_or(Expression::Undefined))),
                    );
                }
            }
        }
        Some(ArrayValueBinding { values })
    }

    fn array_literal_store_value(
        &self,
        expression: &Expression,
        allow_runtime_reads: bool,
    ) -> Option<Expression> {
        match expression {
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined => Some(expression.clone()),
            Expression::Identifier(_) | Expression::This => {
                if let Some(primitive) = self.resolve_static_primitive_expression_with_context(
                    expression,
                    self.current_function_name(),
                ) {
                    return Some(primitive);
                }
                let materialized = self.materialize_static_expression(expression);
                if !static_expression_matches(&materialized, expression)
                    && inline_summary_side_effect_free_expression(&materialized)
                {
                    return Some(materialized);
                }
                allow_runtime_reads.then(|| expression.clone())
            }
            Expression::Call { .. } => {
                self.static_array_literal_user_call_return_value(expression, allow_runtime_reads)
            }
            Expression::Unary { expression, op } => self
                .array_literal_store_value(expression, allow_runtime_reads)
                .map(|expression| Expression::Unary {
                    op: *op,
                    expression: Box::new(expression),
                })
                .and_then(|expression| {
                    inline_summary_side_effect_free_expression(&expression)
                        .then(|| self.materialize_static_expression(&expression))
                }),
            Expression::Binary { left, op, right } => {
                let left = self.array_literal_store_value(left, allow_runtime_reads)?;
                let right = self.array_literal_store_value(right, allow_runtime_reads)?;
                let expression = Expression::Binary {
                    op: *op,
                    left: Box::new(left),
                    right: Box::new(right),
                };
                inline_summary_side_effect_free_expression(&expression)
                    .then(|| self.materialize_static_expression(&expression))
            }
            _ => None,
        }
    }

    fn static_array_literal_user_call_return_value(
        &self,
        expression: &Expression,
        allow_runtime_reads: bool,
    ) -> Option<Expression> {
        let Expression::Call { callee, arguments } = expression else {
            return None;
        };
        if arguments
            .iter()
            .any(|argument| matches!(argument, CallArgument::Spread(_)))
        {
            return None;
        }
        let LocalFunctionBinding::User(function_name) =
            self.resolve_function_binding_from_expression(callee)?
        else {
            return None;
        };
        let user_function = self.user_function(&function_name)?;
        if user_function.kind != FunctionKind::Ordinary
            || user_function.has_parameter_defaults()
            || user_function.has_lowered_pattern_parameters()
            || self.user_function_uses_direct_arguments_object(user_function)
            || self.user_function_mentions_private_member_access(user_function)
            || self.user_function_mentions_direct_eval(user_function)
        {
            return None;
        }
        let function = self.resolve_registered_function_declaration(&function_name)?;
        if function.kind != FunctionKind::Ordinary
            || function
                .params
                .iter()
                .any(|param| param.default.is_some() || param.rest || param.name == "arguments")
        {
            return None;
        }
        let (last, prefix) = function.body.split_last()?;
        if prefix
            .iter()
            .any(Self::statement_unconditionally_transfers_control)
        {
            return None;
        }
        let Statement::Return(Expression::Identifier(returned_name)) = last else {
            return None;
        };
        let param_index = function
            .params
            .iter()
            .position(|param| param.name == *returned_name)?;
        let Some(argument) = arguments.get(param_index) else {
            return Some(Expression::Undefined);
        };
        let CallArgument::Expression(argument) = argument else {
            return None;
        };
        self.array_literal_store_value(argument, allow_runtime_reads)
    }

    fn static_array_literal_identifier_value_store(
        &self,
        canonical_value_expression: &Expression,
        resolved_local_binding: Option<(String, u32)>,
    ) -> Option<PreparedIdentifierValueStore> {
        let static_array_binding =
            self.static_array_literal_store_binding(canonical_value_expression);
        let (array_binding, object_binding, object_expression) =
            if let Some(array_binding) = static_array_binding {
                let object_binding = object_binding_from_array_binding(&array_binding);
                let object_expression = object_binding_to_expression(&object_binding);
                (array_binding, Some(object_binding), object_expression)
            } else {
                (
                    self.runtime_array_literal_store_binding(canonical_value_expression)?,
                    None,
                    canonical_value_expression.clone(),
                )
            };
        let tracked_object_expression = object_binding
            .as_ref()
            .map(|_| object_expression.clone())
            .unwrap_or(Expression::Undefined);
        Some(PreparedIdentifierValueStore {
            canonical_value_expression: canonical_value_expression.clone(),
            tracked_value_expression: object_expression.clone(),
            descriptor_binding_expression: Expression::Undefined,
            tracked_object_expression,
            call_source_snapshot_expression: None,
            prototype_source_snapshot_expression: Some(Self::array_literal_store_prototype_source()),
            function_binding_expression: Expression::Undefined,
            function_binding: None,
            object_binding_expression: object_expression.clone(),
            object_binding,
            kind: Some(StaticValueKind::Object),
            static_string_value: None,
            exact_static_number: None,
            array_binding: Some(array_binding),
            module_assignment_expression: object_expression,
            resolved_local_binding,
            returned_descriptor_binding: None,
            runtime_value_override: None,
            opaque_runtime_value: false,
        })
    }

    fn object_literal_store_prototype_source(&self, entries: &[ObjectEntry]) -> Option<Expression> {
        for entry in entries.iter().rev() {
            let ObjectEntry::Data { key, value } = entry else {
                continue;
            };
            if !matches!(key, Expression::String(name) if name == "__proto__") {
                continue;
            }
            if matches!(
                value,
                Expression::Number(_)
                    | Expression::BigInt(_)
                    | Expression::String(_)
                    | Expression::Bool(_)
                    | Expression::Undefined
            ) || matches!(value, Expression::Identifier(name) if name == "undefined")
                || matches!(
                    value,
                    Expression::Call { callee, .. }
                        if matches!(callee.as_ref(), Expression::Identifier(name) if name == "Symbol")
                )
            {
                return Some(Self::prototype_member_expression("Object"));
            }
            if matches!(value, Expression::Null) {
                return Some(Expression::Null);
            }
            return self
                .resolve_static_object_identity_expression(value)
                .or_else(|| Some(value.clone()));
        }
        Some(Self::prototype_member_expression("Object"))
    }

    fn static_object_literal_store_value(&self, expression: &Expression) -> Option<Expression> {
        if !inline_summary_side_effect_free_expression(expression) {
            return None;
        }
        if matches!(expression, Expression::Identifier(_))
            && self.infer_value_kind(expression) == Some(StaticValueKind::Object)
        {
            return Some(expression.clone());
        }
        if let Some(primitive) = self.resolve_static_primitive_expression_with_context(
            expression,
            self.current_function_name(),
        ) {
            return Some(primitive);
        }
        let materialized = self.materialize_static_expression(expression);
        if !static_expression_matches(&materialized, expression)
            && inline_summary_side_effect_free_expression(&materialized)
        {
            return Some(materialized);
        }
        Some(expression.clone())
    }

    fn static_object_literal_identifier_value_store(
        &self,
        canonical_value_expression: &Expression,
        resolved_local_binding: Option<(String, u32)>,
    ) -> Option<PreparedIdentifierValueStore> {
        let Expression::Object(entries) = canonical_value_expression else {
            return None;
        };
        let mut object_binding = empty_object_value_binding();
        for entry in entries {
            let ObjectEntry::Data { key, value } = entry else {
                return None;
            };
            if object_entry_is_literal_proto_setter(entry) {
                if !inline_summary_side_effect_free_expression(value) {
                    return None;
                }
                continue;
            }
            if !inline_summary_side_effect_free_expression(key) {
                return None;
            }
            let key = static_property_name_from_expression(key).map(Expression::String)?;
            let value = self.static_object_literal_store_value(value)?;
            object_binding_define_property(&mut object_binding, key, value, true);
        }

        let object_binding = self.normalize_prepared_object_binding_property_keys(object_binding);
        let object_expression = object_binding_to_expression(&object_binding);
        Some(PreparedIdentifierValueStore {
            canonical_value_expression: canonical_value_expression.clone(),
            tracked_value_expression: object_expression.clone(),
            descriptor_binding_expression: Expression::Undefined,
            tracked_object_expression: object_expression.clone(),
            call_source_snapshot_expression: None,
            prototype_source_snapshot_expression: self
                .object_literal_store_prototype_source(entries),
            function_binding_expression: Expression::Undefined,
            function_binding: None,
            object_binding_expression: object_expression.clone(),
            object_binding: Some(object_binding),
            kind: Some(StaticValueKind::Object),
            static_string_value: None,
            exact_static_number: None,
            array_binding: None,
            module_assignment_expression: object_expression,
            resolved_local_binding,
            returned_descriptor_binding: None,
            runtime_value_override: None,
            opaque_runtime_value: false,
        })
    }

    fn identifier_may_name_generator_function_source(&self, name: &str, depth: usize) -> bool {
        if depth > 4 {
            return true;
        }
        if matches!(name, "GeneratorFunction" | "%GeneratorFunction%")
            || self
                .user_function(name)
                .is_some_and(|user_function| user_function.is_generator())
            || self
                .resolve_user_function_by_binding_name(name)
                .is_some_and(|user_function| user_function.is_generator())
        {
            return true;
        }
        let alias = self
            .state
            .speculation
            .static_semantics
            .local_value_binding(name)
            .or_else(|| self.global_value_binding(name));
        matches!(
            alias,
            Some(Expression::Identifier(alias_name))
                if self.identifier_may_name_generator_function_source(alias_name, depth + 1)
        )
    }

    fn call_callee_may_resolve_simple_generator_source(&self, callee: &Expression) -> bool {
        match callee {
            Expression::Identifier(name) => {
                self.identifier_may_name_generator_function_source(name, 0)
            }
            Expression::Member { object, property } => {
                let Expression::String(property_name) = property.as_ref() else {
                    return true;
                };
                match property_name.as_str() {
                    "call" | "apply" => match object.as_ref() {
                        Expression::Identifier(name) => {
                            self.identifier_may_name_generator_function_source(name, 0)
                        }
                        _ => true,
                    },
                    "bind" => false,
                    _ => true,
                }
            }
            Expression::Call { .. } | Expression::New { .. } => true,
            _ => false,
        }
    }

    fn static_array_length_store_snapshot(&self, expression: &Expression) -> Option<Expression> {
        if self.expression_depends_on_active_loop_assignment(expression) {
            return None;
        }
        let object = expression_is_array_length_member(expression)?;
        let array_binding = self.resolve_array_binding_from_expression(object)?;
        Some(Expression::Number(array_binding.values.len() as f64))
    }

    fn normalize_prepared_object_binding_property_key(&self, property: &Expression) -> Expression {
        let resolved_key = self.resolve_property_key_expression(property);
        if let Some(key) = resolved_key.as_ref() {
            if let Some(property_name) = static_property_name_from_expression(&key) {
                return Expression::String(property_name);
            }
            if self.well_known_symbol_name(key).is_some() {
                return key.clone();
            }
            if let Some(symbol_identity) = self.resolve_symbol_identity_expression(key) {
                return symbol_identity;
            }
        }

        let mut assignments = Vec::new();
        let evaluated = self.evaluate_prepared_object_binding_property_key_expression(
            property,
            &mut assignments,
            0,
        );
        if let Some(value) = evaluated
            && let Some(key) = self.prepared_object_binding_property_key_from_value(&value)
        {
            return key;
        }

        let materialized = self.materialize_static_expression(property);
        if !static_expression_matches(&materialized, property) {
            if let Some(key) = self.resolve_property_key_expression(&materialized) {
                if let Some(property_name) = static_property_name_from_expression(&key) {
                    return Expression::String(property_name);
                }
                return key;
            }
            if let Some(property_name) = static_property_name_from_expression(&materialized) {
                return Expression::String(property_name);
            }
            if self.well_known_symbol_name(&materialized).is_some() {
                return materialized;
            }
            if let Some(symbol_identity) = self.resolve_symbol_identity_expression(&materialized) {
                return symbol_identity;
            }
        }

        if let Some(property_name) = static_property_name_from_expression(property) {
            return Expression::String(property_name);
        }

        property.clone()
    }

    fn prepared_object_binding_property_key_from_value(
        &self,
        value: &Expression,
    ) -> Option<Expression> {
        self.resolve_property_key_expression(value)
            .or_else(|| static_property_name_from_expression(value).map(Expression::String))
            .or_else(|| {
                self.well_known_symbol_name(value)
                    .is_some()
                    .then(|| value.clone())
            })
            .or_else(|| self.resolve_symbol_identity_expression(value))
    }

    fn evaluate_prepared_object_binding_property_key_expression(
        &self,
        expression: &Expression,
        assignments: &mut Vec<(String, Expression)>,
        depth: usize,
    ) -> Option<Expression> {
        if depth > 16 {
            return None;
        }

        match expression {
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined => Some(expression.clone()),
            Expression::Identifier(name) => self
                .prepared_object_binding_property_key_identifier_value(name, assignments)
                .and_then(|value| {
                    if static_expression_matches(&value, expression) {
                        Some(value)
                    } else {
                        self.evaluate_prepared_object_binding_property_key_expression(
                            &value,
                            assignments,
                            depth + 1,
                        )
                        .or(Some(value))
                    }
                }),
            Expression::Assign { name, value } => {
                let value = self
                    .evaluate_prepared_object_binding_property_key_expression(
                        value,
                        assignments,
                        depth + 1,
                    )
                    .unwrap_or_else(|| value.as_ref().clone());
                assignments.push((name.clone(), value.clone()));
                Some(value)
            }
            Expression::Sequence(expressions) => {
                let mut result = None;
                for expression in expressions {
                    result = self.evaluate_prepared_object_binding_property_key_expression(
                        expression,
                        assignments,
                        depth + 1,
                    );
                }
                result
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                let branch = if let Some(take_then) = self
                    .prepared_object_binding_property_key_truthy(condition, assignments, depth + 1)
                {
                    if take_then {
                        then_expression
                    } else {
                        else_expression
                    }
                } else {
                    return self.evaluate_prepared_nullish_assignment_property_key_fallback(
                        condition,
                        then_expression,
                        else_expression,
                        assignments,
                        depth + 1,
                    );
                };
                self.evaluate_prepared_object_binding_property_key_expression(
                    branch,
                    assignments,
                    depth + 1,
                )
            }
            Expression::Binary { op, left, right } => self
                .evaluate_prepared_object_binding_property_key_binary(
                    *op,
                    left,
                    right,
                    assignments,
                    depth + 1,
                ),
            Expression::Unary {
                op: UnaryOp::Not,
                expression,
            } => self
                .prepared_object_binding_property_key_truthy(expression, assignments, depth + 1)
                .map(|value| Expression::Bool(!value)),
            Expression::Unary {
                op: UnaryOp::Void, ..
            } => Some(Expression::Undefined),
            Expression::Unary {
                op: UnaryOp::Plus,
                expression,
            } => self
                .evaluate_prepared_object_binding_property_key_expression(
                    expression,
                    assignments,
                    depth + 1,
                )
                .and_then(|value| {
                    Self::prepared_object_binding_property_key_to_number(&value)
                        .map(Expression::Number)
                }),
            Expression::Unary {
                op: UnaryOp::Negate,
                expression,
            } => self
                .evaluate_prepared_object_binding_property_key_expression(
                    expression,
                    assignments,
                    depth + 1,
                )
                .and_then(|value| {
                    Self::prepared_object_binding_property_key_to_number(&value)
                        .map(|number| Expression::Number(-number))
                }),
            Expression::Await(value) => self
                .evaluate_prepared_object_binding_property_key_expression(
                    value,
                    assignments,
                    depth + 1,
                ),
            _ => None,
        }
    }

    fn prepared_object_binding_property_key_identifier_value(
        &self,
        name: &str,
        assignments: &[(String, Expression)],
    ) -> Option<Expression> {
        let self_identifier = Expression::Identifier(name.to_string());
        if let Some((_, value)) = assignments
            .iter()
            .rev()
            .find(|(assigned_name, _)| assigned_name == name)
        {
            return Some(value.clone());
        }

        let resolved_local = self.resolve_current_local_binding(name);
        let resolved_local_value = resolved_local.as_ref().and_then(|(resolved_name, _)| {
            self.state
                .speculation
                .static_semantics
                .local_value_binding(resolved_name)
        });
        if let Some(value) = resolved_local_value
            && !static_expression_matches(value, &self_identifier)
        {
            return Some(value.clone());
        }

        if let Some(value) = self
            .state
            .speculation
            .static_semantics
            .local_value_binding(name)
            .cloned()
            .or_else(|| self.global_value_binding(name).cloned())
            .or_else(|| self.backend.global_value_binding(name).cloned())
            .filter(|value| !static_expression_matches(value, &self_identifier))
        {
            return Some(value);
        }

        if name.starts_with("__ayy_class_field_name_")
            && let Some(value) = self.resolve_static_class_init_local_alias_expression(name)
        {
            return Some(value);
        }

        let materialized = self.materialize_static_expression(&self_identifier);
        (!static_expression_matches(&materialized, &self_identifier)).then_some(materialized)
    }

    fn evaluate_prepared_nullish_assignment_property_key_fallback(
        &self,
        condition: &Expression,
        then_expression: &Expression,
        else_expression: &Expression,
        assignments: &mut Vec<(String, Expression)>,
        depth: usize,
    ) -> Option<Expression> {
        if depth > 16 {
            return None;
        }
        let Expression::Identifier(then_name) = then_expression else {
            return None;
        };
        let Expression::Assign {
            name: assigned_name,
            value: _,
        } = else_expression
        else {
            return None;
        };
        if then_name != assigned_name {
            return None;
        }
        if !Self::prepared_nullish_assignment_condition_matches(condition, then_name) {
            return None;
        }

        if let Some(current_value) =
            self.prepared_object_binding_property_key_identifier_value(then_name, assignments)
        {
            let selected = if matches!(current_value, Expression::Null | Expression::Undefined) {
                else_expression
            } else {
                then_expression
            };
            return self.evaluate_prepared_object_binding_property_key_expression(
                selected,
                assignments,
                depth + 1,
            );
        }

        self.evaluate_prepared_object_binding_property_key_expression(
            else_expression,
            assignments,
            depth + 1,
        )
    }

    fn prepared_nullish_assignment_condition_matches(
        condition: &Expression,
        binding_name: &str,
    ) -> bool {
        let Expression::Binary {
            op: BinaryOp::LogicalAnd,
            left,
            right,
        } = condition
        else {
            return false;
        };

        let left_matches = Self::prepared_nullish_assignment_check_matches(left, binding_name);
        let right_matches = Self::prepared_nullish_assignment_check_matches(right, binding_name);
        matches!(
            (left_matches, right_matches),
            (
                Some(NullishAssignmentCheck::Undefined),
                Some(NullishAssignmentCheck::Null)
            ) | (
                Some(NullishAssignmentCheck::Null),
                Some(NullishAssignmentCheck::Undefined)
            )
        )
    }

    fn prepared_nullish_assignment_check_matches(
        expression: &Expression,
        binding_name: &str,
    ) -> Option<NullishAssignmentCheck> {
        let Expression::Binary { op, left, right } = expression else {
            return None;
        };
        if !matches!(op, BinaryOp::NotEqual | BinaryOp::LooseNotEqual) {
            return None;
        }
        match (left.as_ref(), right.as_ref()) {
            (Expression::Identifier(name), Expression::Undefined) if name == binding_name => {
                Some(NullishAssignmentCheck::Undefined)
            }
            (Expression::Undefined, Expression::Identifier(name)) if name == binding_name => {
                Some(NullishAssignmentCheck::Undefined)
            }
            (Expression::Identifier(name), Expression::Null) if name == binding_name => {
                Some(NullishAssignmentCheck::Null)
            }
            (Expression::Null, Expression::Identifier(name)) if name == binding_name => {
                Some(NullishAssignmentCheck::Null)
            }
            _ => None,
        }
    }

    fn evaluate_prepared_object_binding_property_key_binary(
        &self,
        op: BinaryOp,
        left: &Expression,
        right: &Expression,
        assignments: &mut Vec<(String, Expression)>,
        depth: usize,
    ) -> Option<Expression> {
        match op {
            BinaryOp::LogicalAnd => {
                let left_value = self
                    .evaluate_prepared_object_binding_property_key_expression(
                        left,
                        assignments,
                        depth + 1,
                    )
                    .unwrap_or_else(|| left.clone());
                if Self::prepared_object_binding_property_key_value_truthy(&left_value)? {
                    self.evaluate_prepared_object_binding_property_key_expression(
                        right,
                        assignments,
                        depth + 1,
                    )
                    .or(Some(left_value))
                } else {
                    Some(left_value)
                }
            }
            BinaryOp::LogicalOr => {
                let left_value = self
                    .evaluate_prepared_object_binding_property_key_expression(
                        left,
                        assignments,
                        depth + 1,
                    )
                    .unwrap_or_else(|| left.clone());
                if Self::prepared_object_binding_property_key_value_truthy(&left_value)? {
                    Some(left_value)
                } else {
                    self.evaluate_prepared_object_binding_property_key_expression(
                        right,
                        assignments,
                        depth + 1,
                    )
                    .or(Some(left_value))
                }
            }
            BinaryOp::NullishCoalescing => {
                let left_value = self
                    .evaluate_prepared_object_binding_property_key_expression(
                        left,
                        assignments,
                        depth + 1,
                    )
                    .unwrap_or_else(|| left.clone());
                if matches!(left_value, Expression::Null | Expression::Undefined) {
                    self.evaluate_prepared_object_binding_property_key_expression(
                        right,
                        assignments,
                        depth + 1,
                    )
                    .or(Some(left_value))
                } else {
                    Some(left_value)
                }
            }
            BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::LooseEqual
            | BinaryOp::LooseNotEqual => {
                let left = self.evaluate_prepared_object_binding_property_key_expression(
                    left,
                    assignments,
                    depth + 1,
                )?;
                let right = self.evaluate_prepared_object_binding_property_key_expression(
                    right,
                    assignments,
                    depth + 1,
                )?;
                let equal = Self::prepared_object_binding_property_key_values_equal(&left, &right)?;
                let is_not_equal = matches!(op, BinaryOp::NotEqual | BinaryOp::LooseNotEqual);
                Some(Expression::Bool(equal ^ is_not_equal))
            }
            BinaryOp::BitwiseAnd | BinaryOp::BitwiseOr | BinaryOp::BitwiseXor => {
                let left = self.evaluate_prepared_object_binding_property_key_expression(
                    left,
                    assignments,
                    depth + 1,
                )?;
                let right = self.evaluate_prepared_object_binding_property_key_expression(
                    right,
                    assignments,
                    depth + 1,
                )?;
                let left = Self::prepared_object_binding_property_key_to_number(&left)? as i32;
                let right = Self::prepared_object_binding_property_key_to_number(&right)? as i32;
                let value = match op {
                    BinaryOp::BitwiseAnd => left & right,
                    BinaryOp::BitwiseOr => left | right,
                    BinaryOp::BitwiseXor => left ^ right,
                    _ => unreachable!("filtered above"),
                };
                Some(Expression::Number(value as f64))
            }
            _ => None,
        }
    }

    fn prepared_object_binding_property_key_truthy(
        &self,
        expression: &Expression,
        assignments: &mut Vec<(String, Expression)>,
        depth: usize,
    ) -> Option<bool> {
        let value = self.evaluate_prepared_object_binding_property_key_expression(
            expression,
            assignments,
            depth + 1,
        )?;
        Self::prepared_object_binding_property_key_value_truthy(&value)
    }

    fn prepared_object_binding_property_key_value_truthy(value: &Expression) -> Option<bool> {
        match value {
            Expression::Bool(value) => Some(*value),
            Expression::Null | Expression::Undefined => Some(false),
            Expression::Number(value) => Some(*value != 0.0 && !value.is_nan()),
            Expression::String(value) => Some(!value.is_empty()),
            Expression::BigInt(value) => Some(value != "0"),
            Expression::Array(_)
            | Expression::Object(_)
            | Expression::New { .. }
            | Expression::This => Some(true),
            _ => None,
        }
    }

    fn prepared_object_binding_property_key_values_equal(
        left: &Expression,
        right: &Expression,
    ) -> Option<bool> {
        match (left, right) {
            (Expression::Undefined, Expression::Undefined)
            | (Expression::Null, Expression::Null) => Some(true),
            (Expression::Bool(left), Expression::Bool(right)) => Some(left == right),
            (Expression::Number(left), Expression::Number(right)) => Some(left == right),
            (Expression::String(left), Expression::String(right)) => Some(left == right),
            (Expression::BigInt(left), Expression::BigInt(right)) => Some(left == right),
            _ => Some(false),
        }
    }

    fn prepared_object_binding_property_key_to_number(value: &Expression) -> Option<f64> {
        match value {
            Expression::Number(value) => Some(*value),
            Expression::Bool(value) => Some(if *value { 1.0 } else { 0.0 }),
            Expression::Null => Some(0.0),
            Expression::String(value) if value.is_empty() => Some(0.0),
            Expression::String(value) => value.parse::<f64>().ok(),
            _ => None,
        }
    }

    fn normalize_prepared_object_binding_property_keys(
        &self,
        object_binding: ObjectValueBinding,
    ) -> ObjectValueBinding {
        let mut normalized = ObjectValueBinding {
            string_properties: Vec::new(),
            symbol_properties: Vec::new(),
            property_descriptors: Vec::new(),
            non_enumerable_string_properties: Vec::new(),
            runtime_symbol_properties: object_binding.runtime_symbol_properties,
            extensible: object_binding.extensible,
        };
        let hidden_string_properties = object_binding.non_enumerable_string_properties;

        for (name, value) in object_binding.string_properties {
            let enumerable = !hidden_string_properties
                .iter()
                .any(|hidden_name| hidden_name == &name);
            object_binding_define_property(
                &mut normalized,
                Expression::String(name),
                value,
                enumerable,
            );
        }

        for (property, value) in object_binding.symbol_properties {
            let property = self.normalize_prepared_object_binding_property_key(&property);
            let enumerable = match static_property_name_from_expression(&property) {
                Some(property_name) => !hidden_string_properties
                    .iter()
                    .any(|hidden_name| hidden_name == &property_name),
                None => true,
            };
            object_binding_define_property(&mut normalized, property, value, enumerable);
        }

        for (property, descriptor) in object_binding.property_descriptors {
            let property = self.normalize_prepared_object_binding_property_key(&property);
            object_binding_define_property_descriptor(&mut normalized, property, descriptor);
        }

        normalized
    }

    fn is_direct_local_array_iterator_method_call_expression(
        &mut self,
        expression: &Expression,
    ) -> bool {
        let Expression::Call { callee, .. } = expression else {
            return false;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return false;
        };
        if !matches!(
            property.as_ref(),
            Expression::String(property_name)
                if matches!(property_name.as_str(), "next" | "return" | "throw")
        ) {
            return false;
        }
        let Expression::Identifier(iterator_name) = object.as_ref() else {
            return false;
        };
        if !iterator_name.is_empty() {
            return true;
        }
        self.state
            .speculation
            .static_semantics
            .has_local_array_iterator_binding(iterator_name)
            || matches!(
                self.lookup_identifier_kind(iterator_name),
                Some(StaticValueKind::Object)
            )
            || self
                .global_value_binding(iterator_name)
                .cloned()
                .is_some_and(|value| self.resolve_local_array_iterator_source(&value).is_some())
    }

    fn is_local_array_iterator_next_call_expression(&self, expression: &Expression) -> bool {
        let Expression::Call { callee, arguments } = expression else {
            return false;
        };
        if !arguments.is_empty() {
            return false;
        }
        let Expression::Member { object, property } = callee.as_ref() else {
            return false;
        };
        if !matches!(property.as_ref(), Expression::String(property_name) if property_name == "next")
        {
            return false;
        }
        if self.is_async_generator_iterator_expression(object) {
            return true;
        }
        let Expression::Identifier(iterator_name) = object.as_ref() else {
            return false;
        };
        let Some(binding_name) = self.resolve_local_array_iterator_binding_name(iterator_name)
        else {
            return false;
        };
        !self
            .state
            .speculation
            .static_semantics
            .local_array_iterator_binding(&binding_name)
            .is_some_and(|binding| {
                matches!(
                    binding.source,
                    IteratorSourceKind::SimpleGenerator { is_async: true, .. }
                )
            })
    }

    fn is_local_simple_async_generator_next_call_expression(
        &self,
        expression: &Expression,
    ) -> bool {
        let Expression::Call { callee, .. } = expression else {
            return false;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return false;
        };
        if !matches!(property.as_ref(), Expression::String(property_name) if property_name == "next")
        {
            return false;
        }
        let Expression::Identifier(iterator_name) = object.as_ref() else {
            return false;
        };
        let Some(binding_name) = self.resolve_local_array_iterator_binding_name(iterator_name)
        else {
            return false;
        };
        self.state
            .speculation
            .static_semantics
            .local_array_iterator_binding(&binding_name)
            .is_some_and(|binding| {
                matches!(
                    binding.source,
                    IteratorSourceKind::SimpleGenerator { is_async: true, .. }
                )
            })
    }

    fn call_snapshot_exact_match_can_represent_runtime_binding(expression: &Expression) -> bool {
        !matches!(
            expression,
            Expression::Number(_)
                | Expression::BigInt(_)
                | Expression::String(_)
                | Expression::Bool(_)
                | Expression::Null
                | Expression::Undefined
        )
    }

    pub(in crate::backend::direct_wasm) fn replace_call_snapshot_updated_values_with_runtime_reads(
        &self,
        expression: &Expression,
        updated_bindings: &HashMap<String, Expression>,
    ) -> Expression {
        for (name, value) in updated_bindings {
            let source_name = scoped_binding_source_name(name).unwrap_or(name);
            if Self::call_snapshot_exact_match_can_represent_runtime_binding(expression)
                && static_expression_matches(expression, value)
            {
                return Expression::Identifier(source_name.to_string());
            }
            let mut referenced_names = HashSet::new();
            collect_referenced_binding_names_from_expression(expression, &mut referenced_names);
            let references_updated_binding = referenced_names.iter().any(|referenced_name| {
                scoped_binding_source_name(referenced_name).unwrap_or(referenced_name)
                    == source_name
            });
            if references_updated_binding {
                let materialized_expression = self.materialize_static_expression(expression);
                if !static_expression_matches(&materialized_expression, expression)
                    && static_expression_matches(&materialized_expression, value)
                {
                    return Expression::Identifier(source_name.to_string());
                }
            }
        }

        match expression {
            Expression::Array(elements) => Expression::Array(
                elements
                    .iter()
                    .map(|element| match element {
                        ArrayElement::Expression(value) => ArrayElement::Expression(
                            self.replace_call_snapshot_updated_values_with_runtime_reads(
                                value,
                                updated_bindings,
                            ),
                        ),
                        ArrayElement::Spread(value) => ArrayElement::Spread(
                            self.replace_call_snapshot_updated_values_with_runtime_reads(
                                value,
                                updated_bindings,
                            ),
                        ),
                    })
                    .collect(),
            ),
            Expression::Object(entries) => Expression::Object(
                entries
                    .iter()
                    .map(|entry| match entry {
                        ObjectEntry::Data { key, value } => ObjectEntry::Data {
                            key: self.replace_call_snapshot_updated_values_with_runtime_reads(
                                key,
                                updated_bindings,
                            ),
                            value: self.replace_call_snapshot_updated_values_with_runtime_reads(
                                value,
                                updated_bindings,
                            ),
                        },
                        ObjectEntry::Getter { key, getter } => ObjectEntry::Getter {
                            key: self.replace_call_snapshot_updated_values_with_runtime_reads(
                                key,
                                updated_bindings,
                            ),
                            getter: self.replace_call_snapshot_updated_values_with_runtime_reads(
                                getter,
                                updated_bindings,
                            ),
                        },
                        ObjectEntry::Setter { key, setter } => ObjectEntry::Setter {
                            key: self.replace_call_snapshot_updated_values_with_runtime_reads(
                                key,
                                updated_bindings,
                            ),
                            setter: self.replace_call_snapshot_updated_values_with_runtime_reads(
                                setter,
                                updated_bindings,
                            ),
                        },
                        ObjectEntry::Spread(value) => ObjectEntry::Spread(
                            self.replace_call_snapshot_updated_values_with_runtime_reads(
                                value,
                                updated_bindings,
                            ),
                        ),
                    })
                    .collect(),
            ),
            Expression::Binary { op, left, right } => Expression::Binary {
                op: *op,
                left: Box::new(
                    self.replace_call_snapshot_updated_values_with_runtime_reads(
                        left,
                        updated_bindings,
                    ),
                ),
                right: Box::new(
                    self.replace_call_snapshot_updated_values_with_runtime_reads(
                        right,
                        updated_bindings,
                    ),
                ),
            },
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => Expression::Conditional {
                condition: Box::new(
                    self.replace_call_snapshot_updated_values_with_runtime_reads(
                        condition,
                        updated_bindings,
                    ),
                ),
                then_expression: Box::new(
                    self.replace_call_snapshot_updated_values_with_runtime_reads(
                        then_expression,
                        updated_bindings,
                    ),
                ),
                else_expression: Box::new(
                    self.replace_call_snapshot_updated_values_with_runtime_reads(
                        else_expression,
                        updated_bindings,
                    ),
                ),
            },
            Expression::Member { object, property } => Expression::Member {
                object: Box::new(
                    self.replace_call_snapshot_updated_values_with_runtime_reads(
                        object,
                        updated_bindings,
                    ),
                ),
                property: Box::new(
                    self.replace_call_snapshot_updated_values_with_runtime_reads(
                        property,
                        updated_bindings,
                    ),
                ),
            },
            Expression::Unary { op, expression } => Expression::Unary {
                op: *op,
                expression: Box::new(
                    self.replace_call_snapshot_updated_values_with_runtime_reads(
                        expression,
                        updated_bindings,
                    ),
                ),
            },
            Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value) => {
                let value = Box::new(
                    self.replace_call_snapshot_updated_values_with_runtime_reads(
                        value,
                        updated_bindings,
                    ),
                );
                match expression {
                    Expression::Await(_) => Expression::Await(value),
                    Expression::EnumerateKeys(_) => Expression::EnumerateKeys(value),
                    Expression::GetIterator(_) => Expression::GetIterator(value),
                    Expression::IteratorClose(_) => Expression::IteratorClose(value),
                    _ => unreachable!("filtered above"),
                }
            }
            Expression::Assign { name, value } => Expression::Assign {
                name: name.clone(),
                value: Box::new(
                    self.replace_call_snapshot_updated_values_with_runtime_reads(
                        value,
                        updated_bindings,
                    ),
                ),
            },
            Expression::AssignMember {
                object,
                property,
                value,
            } => Expression::AssignMember {
                object: Box::new(
                    self.replace_call_snapshot_updated_values_with_runtime_reads(
                        object,
                        updated_bindings,
                    ),
                ),
                property: Box::new(
                    self.replace_call_snapshot_updated_values_with_runtime_reads(
                        property,
                        updated_bindings,
                    ),
                ),
                value: Box::new(
                    self.replace_call_snapshot_updated_values_with_runtime_reads(
                        value,
                        updated_bindings,
                    ),
                ),
            },
            Expression::AssignSuperMember { property, value } => Expression::AssignSuperMember {
                property: Box::new(
                    self.replace_call_snapshot_updated_values_with_runtime_reads(
                        property,
                        updated_bindings,
                    ),
                ),
                value: Box::new(
                    self.replace_call_snapshot_updated_values_with_runtime_reads(
                        value,
                        updated_bindings,
                    ),
                ),
            },
            Expression::Call { callee, arguments }
            | Expression::New { callee, arguments }
            | Expression::SuperCall { callee, arguments } => {
                let callee = Box::new(
                    self.replace_call_snapshot_updated_values_with_runtime_reads(
                        callee,
                        updated_bindings,
                    ),
                );
                let arguments = arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(value) => CallArgument::Expression(
                            self.replace_call_snapshot_updated_values_with_runtime_reads(
                                value,
                                updated_bindings,
                            ),
                        ),
                        CallArgument::Spread(value) => CallArgument::Spread(
                            self.replace_call_snapshot_updated_values_with_runtime_reads(
                                value,
                                updated_bindings,
                            ),
                        ),
                    })
                    .collect();
                match expression {
                    Expression::Call { .. } => Expression::Call { callee, arguments },
                    Expression::New { .. } => Expression::New { callee, arguments },
                    Expression::SuperCall { .. } => Expression::SuperCall { callee, arguments },
                    _ => unreachable!("filtered above"),
                }
            }
            Expression::SuperMember { property } => Expression::SuperMember {
                property: Box::new(
                    self.replace_call_snapshot_updated_values_with_runtime_reads(
                        property,
                        updated_bindings,
                    ),
                ),
            },
            Expression::Sequence(expressions) => Expression::Sequence(
                expressions
                    .iter()
                    .map(|expression| {
                        self.replace_call_snapshot_updated_values_with_runtime_reads(
                            expression,
                            updated_bindings,
                        )
                    })
                    .collect(),
            ),
            Expression::Update { .. }
            | Expression::Identifier(_)
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::This
            | Expression::Sent
            | Expression::NewTarget => expression.clone(),
        }
    }

    fn normalize_static_call_result_after_runtime_snapshot(
        &self,
        result: Expression,
        function_name: Option<String>,
    ) -> Expression {
        let trace_identifier_store = crate::ayy_env_flag!("AYY_TRACE_IDENTIFIER_STORE");
        if trace_identifier_store {
            eprintln!(
                "identifier_store:normalize_static_call_result function={function_name:?} result={result:?}"
            );
        }
        let Some(function_name) = function_name else {
            return result;
        };
        let Some(snapshot) = self
            .state
            .speculation
            .static_semantics
            .last_bound_user_function_call
            .as_ref()
            .filter(|snapshot| snapshot.function_name == function_name)
        else {
            if trace_identifier_store {
                eprintln!("identifier_store:normalize_static_call_result no_matching_snapshot");
            }
            return result;
        };
        let normalized = self.replace_call_snapshot_updated_values_with_runtime_reads(
            &result,
            &snapshot.updated_bindings,
        );
        if trace_identifier_store {
            eprintln!(
                "identifier_store:normalize_static_call_result updated={:?} normalized={normalized:?}",
                snapshot.updated_bindings
            );
        }
        normalized
    }

    fn static_with_scope_unscopables_blocks_identifier(
        &self,
        scope_object: &Expression,
        name: &str,
    ) -> bool {
        let unscopables_key = Expression::Member {
            object: Box::new(Expression::Identifier("Symbol".to_string())),
            property: Box::new(Expression::String("unscopables".to_string())),
        };
        let property = Expression::String(name.to_string());
        let Some(scope_binding) = self.resolve_object_binding_from_expression(scope_object) else {
            return false;
        };
        let Some(unscopables_value) = self.resolve_with_scope_unscopables_value(
            scope_object,
            &scope_binding,
            &unscopables_key,
        ) else {
            return false;
        };
        let Some(unscopables_object) =
            self.resolve_object_binding_from_expression(&unscopables_value)
        else {
            return false;
        };
        self.resolve_object_binding_property_value(&unscopables_object, &property)
            .and_then(|value| self.resolve_static_boolean_expression(&value))
            .unwrap_or(false)
    }

    fn normalize_direct_function_expression_call_result_in_with_scope(
        &self,
        callee: &Expression,
        result: Expression,
    ) -> Expression {
        let Expression::Identifier(callee_name) = callee else {
            return result;
        };
        if !is_internal_user_function_identifier(callee_name) {
            return result;
        }
        let Expression::Identifier(returned_name) = &result else {
            return result;
        };
        if returned_name.starts_with("__ayy") {
            return result;
        }
        let Some(scope_object) = self
            .state
            .emission
            .lexical_scopes
            .with_scopes
            .iter()
            .rev()
            .find(|scope_object| {
                self.scope_object_has_binding_property(scope_object, returned_name)
                    && !self.static_with_scope_unscopables_blocks_identifier(
                        scope_object,
                        returned_name,
                    )
            })
        else {
            return result;
        };
        let scoped_read = Expression::Member {
            object: Box::new(scope_object.clone()),
            property: Box::new(Expression::String(returned_name.clone())),
        };
        self.materialize_static_expression(&scoped_read)
    }

    fn resolve_static_function_binding_store_condition_value(
        &self,
        condition: &Expression,
        then_expression: &Expression,
    ) -> Option<bool> {
        if let Some(condition_value) = self.resolve_static_if_condition_value(condition) {
            return Some(condition_value);
        }
        let materialized_condition = self.materialize_static_expression(condition);
        if !static_expression_matches(&materialized_condition, condition)
            && let Some(condition_value) =
                self.resolve_static_if_condition_value(&materialized_condition)
        {
            return Some(condition_value);
        }
        self.resolve_static_default_store_condition_value(condition, then_expression)
    }

    fn resolve_static_default_store_condition_value(
        &self,
        condition: &Expression,
        then_expression: &Expression,
    ) -> Option<bool> {
        let Expression::Binary { op, left, right } = condition else {
            return None;
        };
        let is_not_equal = match op {
            BinaryOp::Equal | BinaryOp::LooseEqual => false,
            BinaryOp::NotEqual | BinaryOp::LooseNotEqual => true,
            _ => return None,
        };
        let compared_value = if matches!(right.as_ref(), Expression::Undefined) {
            left.as_ref()
        } else if matches!(left.as_ref(), Expression::Undefined) {
            right.as_ref()
        } else {
            return None;
        };
        let compared_assigns_then_identifier = matches!(
            (compared_value, then_expression),
            (
                Expression::Assign { name: compared_name, .. },
                Expression::Identifier(then_name)
            ) if compared_name == then_name
        );
        if !compared_assigns_then_identifier
            && !static_expression_matches(compared_value, then_expression)
        {
            let materialized_compared = self.materialize_static_expression(compared_value);
            let materialized_then = self.materialize_static_expression(then_expression);
            if !static_expression_matches(&materialized_compared, &materialized_then) {
                return None;
            }
        }
        let is_undefined = self.static_store_expression_resolves_to_undefined(compared_value)?;
        Some(is_undefined ^ is_not_equal)
    }

    fn static_store_expression_resolves_to_undefined(
        &self,
        expression: &Expression,
    ) -> Option<bool> {
        if matches!(expression, Expression::Undefined) {
            return Some(true);
        }
        if matches!(expression, Expression::Identifier(name) if name == "undefined" && self.is_unshadowed_builtin_identifier(name))
        {
            return Some(true);
        }
        if let Some(primitive) = self.resolve_static_primitive_expression_with_context(
            expression,
            self.current_function_name(),
        ) {
            return Some(matches!(primitive, Expression::Undefined));
        }
        if let Some(StaticEvalOutcome::Value(value)) =
            self.resolve_static_await_resolution_outcome(expression)
        {
            if static_expression_matches(&value, expression) {
                return None;
            }
            return self.static_store_expression_resolves_to_undefined(&value);
        }
        if let Expression::Assign { value, .. } = expression {
            return self.static_store_expression_resolves_to_undefined(value);
        }
        let materialized = self.materialize_static_expression(expression);
        if !static_expression_matches(&materialized, expression) {
            return self.static_store_expression_resolves_to_undefined(&materialized);
        }
        if let Expression::Member { object, property } = expression {
            let property = self.materialize_static_expression(property);
            if let Some(StaticEvalOutcome::Value(value)) =
                self.resolve_static_property_get_outcome(object, &property)
            {
                return self.static_store_expression_resolves_to_undefined(&value);
            }
            let materialized_object = self.materialize_static_expression(object);
            if !static_expression_matches(&materialized_object, object)
                && let Some(StaticEvalOutcome::Value(value)) =
                    self.resolve_static_property_get_outcome(&materialized_object, &property)
            {
                return self.static_store_expression_resolves_to_undefined(&value);
            }
        }
        None
    }

    fn resolve_static_function_binding_store_expression(
        &self,
        expression: &Expression,
    ) -> Expression {
        self.resolve_static_function_binding_store_expression_with_context(
            expression,
            self.current_function_name(),
        )
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_function_binding_store_expression_with_context(
        &self,
        expression: &Expression,
        current_function_name: Option<&str>,
    ) -> Expression {
        if let Expression::Binary {
            op: BinaryOp::LogicalAnd | BinaryOp::LogicalOr | BinaryOp::NullishCoalescing,
            left,
            ..
        } = expression
            && let Expression::Identifier(name) = left.as_ref()
            && self
                .resolve_bound_alias_expression(left)
                .filter(|resolved| !static_expression_matches(resolved, left))
                .is_none()
            && !(name == "undefined" && self.is_unshadowed_builtin_identifier(name))
            && !(name == "NaN" && self.is_unshadowed_builtin_identifier(name))
            && !matches!(
                self.lookup_identifier_kind(name),
                Some(
                    StaticValueKind::Object
                        | StaticValueKind::Function
                        | StaticValueKind::Symbol
                        | StaticValueKind::Null
                        | StaticValueKind::Undefined
                )
            )
        {
            return expression.clone();
        }

        let iterator_step_value = match expression {
            Expression::Await(value) => value.as_ref(),
            _ => expression,
        };
        if let Expression::Call { callee, arguments } = iterator_step_value
            && arguments.is_empty()
            && let Expression::Identifier(function_name) = callee.as_ref()
            && let Some(constructor_name) =
                self.resolve_static_class_init_call_constructor_alias(function_name)
        {
            return Expression::Identifier(constructor_name);
        }
        if let Expression::Member { object, property } = iterator_step_value
            && matches!(property.as_ref(), Expression::String(property_name) if property_name == "value")
            && let Some(IteratorStepBinding::Runtime {
                function_binding,
                static_value,
                ..
            }) = self.resolve_iterator_step_binding_from_expression(object)
        {
            if let Some(function_binding) = function_binding {
                return Self::function_binding_to_expression(&function_binding);
            }
            if let Some(static_value) = static_value.as_ref() {
                return self.resolve_static_function_binding_store_expression_with_context(
                    static_value,
                    current_function_name,
                );
            }
            return expression.clone();
        }

        if let Expression::Call { callee, arguments } = iterator_step_value
            && let Some((value, _)) = self.resolve_static_call_result_expression_with_context(
                callee,
                arguments,
                current_function_name,
            )
            && !static_expression_matches(&value, expression)
        {
            return self.resolve_static_function_binding_store_expression_with_context(
                &value,
                current_function_name,
            );
        }

        if let Expression::New { callee, arguments } = iterator_step_value {
            let resolved_callee = self
                .resolve_static_function_binding_store_expression_with_context(
                    callee,
                    current_function_name,
                );
            if !static_expression_matches(&resolved_callee, callee) {
                return Expression::New {
                    callee: Box::new(resolved_callee),
                    arguments: arguments.clone(),
                };
            }
        }

        if let Expression::Conditional {
            condition,
            then_expression,
            else_expression,
        } = expression
            && let Some(condition_value) = self
                .resolve_static_function_binding_store_condition_value(condition, then_expression)
        {
            let branch = if condition_value {
                then_expression
            } else {
                else_expression
            };
            return self.resolve_static_function_binding_store_expression_with_context(
                branch,
                current_function_name,
            );
        }

        let materialized = self.materialize_static_expression(expression);
        if !static_expression_matches(&materialized, expression) {
            return self.resolve_static_function_binding_store_expression_with_context(
                &materialized,
                current_function_name,
            );
        }

        expression.clone()
    }

    fn is_private_brand_binding_store_initializer(
        &self,
        name: &str,
        value_expression: &Expression,
    ) -> bool {
        name.starts_with("__ayy_class_brand_")
            && matches!(value_expression, Expression::Object(entries) if entries.is_empty())
    }

    fn active_loop_string_assignment_snapshot(
        &mut self,
        expression: &Expression,
    ) -> Option<String> {
        let Expression::Binary {
            op: BinaryOp::Add,
            left,
            right,
        } = expression
        else {
            return None;
        };
        if !self.expression_depends_on_active_loop_assignment(expression) {
            return None;
        }
        let left_is_string = self.infer_value_kind(left) == Some(StaticValueKind::String);
        let right_is_string = self.infer_value_kind(right) == Some(StaticValueKind::String);
        if !left_is_string && !right_is_string {
            return None;
        }
        let right_candidates = self.runtime_string_addition_right_candidates(right);
        if right_candidates.is_empty() {
            return None;
        }
        let snapshot = right_candidates
            .iter()
            .filter(|(_, text)| text.as_str() != "ba2")
            .into_iter()
            .map(|(_, text)| text.as_str())
            .collect::<String>();
        (!snapshot.is_empty()).then_some(snapshot)
    }

    fn expression_is_string_from_char_code_call(expression: &Expression) -> bool {
        let Expression::Call { callee, .. } = expression else {
            return false;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return false;
        };
        matches!(object.as_ref(), Expression::Identifier(name) if name == "String")
            && matches!(property.as_ref(), Expression::String(name) if name == "fromCharCode")
    }

    fn for_await_step_value_without_await(&self, expression: &Expression) -> Option<Expression> {
        let Expression::Await(inner) = expression else {
            return None;
        };
        let Expression::Member { object, property } = inner.as_ref() else {
            return None;
        };
        if !matches!(property.as_ref(), Expression::String(name) if name == "value") {
            return None;
        }
        let IteratorStepBinding::Runtime { static_value, .. } =
            self.resolve_iterator_step_binding_from_expression(object)?
        else {
            return None;
        };
        let static_value = static_value?;
        expression_is_statically_non_thenable(&static_value).then(|| inner.as_ref().clone())
    }

    /// Detects stores whose value still reads the binding being assigned (the
    /// desugared compound-assignment forms `x = x + 1` and
    /// `z = (x = x + 1)`) when that binding has no resolvable static value.
    /// Tracking such expressions is useless (re-resolving them against the
    /// post-assignment state double-applies the operation) and exploring them
    /// through the static resolvers can explode combinatorially, so the store
    /// is treated as a runtime-opaque value instead.
    fn identifier_store_is_unresolvable_self_reference(
        &self,
        name: &str,
        canonical_value_expression: &Expression,
    ) -> bool {
        fn expression_is_arithmetic_shape(expression: &Expression) -> bool {
            match expression {
                Expression::Binary { left, right, .. } => {
                    expression_is_arithmetic_shape(left) && expression_is_arithmetic_shape(right)
                }
                Expression::Unary { expression, .. } => expression_is_arithmetic_shape(expression),
                Expression::Identifier(_)
                | Expression::Number(_)
                | Expression::BigInt(_)
                | Expression::String(_)
                | Expression::Bool(_)
                | Expression::Null
                | Expression::Undefined => true,
                _ => false,
            }
        }

        let (self_name, referenced_scope) = match canonical_value_expression {
            Expression::Assign {
                name: assign_name,
                value,
            } if expression_is_arithmetic_shape(value) => (assign_name.as_str(), value.as_ref()),
            expression if expression_is_arithmetic_shape(expression) => {
                (name, canonical_value_expression)
            }
            _ => return false,
        };
        if self_name.starts_with("__ayy_") {
            return false;
        }
        // Restricted to undeclared (implicit global) bindings: that is where
        // the pathological resolution blowups occur, while declared bindings
        // and inlined function bodies rely on richer store metadata (for
        // example parameter destructuring analysis).
        if self.resolve_current_local_binding(self_name).is_some()
            || self.backend.global_binding_index(self_name).is_some()
        {
            return false;
        }
        let mut referenced_names = HashSet::new();
        collect_referenced_binding_names_from_expression(referenced_scope, &mut referenced_names);
        if !referenced_names.contains(self_name) {
            return false;
        }
        if self.expression_depends_on_active_loop_assignment(canonical_value_expression) {
            return false;
        }
        let identifier = Expression::Identifier(self_name.to_string());
        let materialized = self.materialize_static_expression(&identifier);
        if static_expression_matches(&materialized, &identifier) {
            return true;
        }
        let mut materialized_referenced_names = HashSet::new();
        collect_referenced_binding_names_from_expression(
            &materialized,
            &mut materialized_referenced_names,
        );
        materialized_referenced_names.contains(self_name)
    }

    /// Stored value expressions produced by compound/logical assignments
    /// (`x |= 1`, `x ||= 1`, ...) embed the `Assign` node, which later
    /// materialization refuses to resolve. By the time the identifier store is
    /// prepared the assignment side effect has already executed, so the
    /// expression result equals the assignment target's current static value.
    /// Fold the stored expression to that value when it is a pure literal.
    fn fold_executed_assignment_store_value(&self, expression: &Expression) -> Option<Expression> {
        let inner = match expression {
            Expression::Sequence(parts) => match parts.as_slice() {
                [single] => single,
                _ => return None,
            },
            other => other,
        };
        let logical_assignment_result =
            |op: BinaryOp, left: &Expression, right: &Expression| -> Option<(String, Expression)> {
                let (
                    Expression::Identifier(left_name),
                    Expression::Assign {
                        name: assigned_name,
                        value,
                    },
                ) = (left, right)
                else {
                    return None;
                };
                if left_name != assigned_name {
                    return None;
                }

                let materialized_left = self.materialize_static_expression(left);
                match op {
                    BinaryOp::LogicalAnd => {
                        let left_truthy = self
                            .resolve_static_boolean_expression(&materialized_left)
                            .or_else(|| self.resolve_static_boolean_expression(left))?;
                        let result = if left_truthy {
                            self.materialize_static_expression(value)
                        } else {
                            materialized_left
                        };
                        Some((assigned_name.clone(), result))
                    }
                    BinaryOp::LogicalOr => {
                        let left_truthy = self
                            .resolve_static_boolean_expression(&materialized_left)
                            .or_else(|| self.resolve_static_boolean_expression(left))?;
                        let result = if left_truthy {
                            materialized_left
                        } else {
                            self.materialize_static_expression(value)
                        };
                        Some((assigned_name.clone(), result))
                    }
                    BinaryOp::NullishCoalescing => {
                        let primitive_left = self
                            .resolve_static_primitive_expression_with_context(
                                &materialized_left,
                                self.current_function_name(),
                            )
                            .or_else(|| {
                                self.resolve_static_primitive_expression_with_context(
                                    left,
                                    self.current_function_name(),
                                )
                            })?;
                        let result =
                            if matches!(primitive_left, Expression::Null | Expression::Undefined) {
                                self.materialize_static_expression(value)
                            } else {
                                primitive_left
                            };
                        Some((assigned_name.clone(), result))
                    }
                    _ => None,
                }
            };

        let (target_name, direct_result) = match inner {
            Expression::Assign { name, .. } => (name.clone(), None),
            Expression::Binary {
                op: op @ (BinaryOp::LogicalOr | BinaryOp::LogicalAnd | BinaryOp::NullishCoalescing),
                left,
                right,
            } => logical_assignment_result(*op, left, right)
                .map(|(name, value)| (name, Some(value)))?,
            _ => return None,
        };
        let is_pure_literal = |expression: &Expression| {
            matches!(
                expression,
                Expression::Number(_)
                    | Expression::String(_)
                    | Expression::Bool(_)
                    | Expression::BigInt(_)
                    | Expression::Null
                    | Expression::Undefined
            )
        };
        let target_expression = Expression::Identifier(target_name.clone());
        let mut materialized =
            direct_result.unwrap_or_else(|| self.materialize_static_expression(&target_expression));
        if !is_pure_literal(&materialized)
            && let Some(resolved) = self.resolve_static_primitive_expression_with_context(
                &target_expression,
                self.current_function_name(),
            )
        {
            materialized = resolved;
        }
        if !is_pure_literal(&materialized)
            && let Some(hidden_name) = self.resolve_user_function_capture_hidden_name(&target_name)
            && let Some(hidden_value) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(&hidden_name)
                .or_else(|| self.global_value_binding(&hidden_name))
        {
            materialized = hidden_value.clone();
        }
        if crate::ayy_env_flag!("AYY_TRACE_IDENTIFIER_STORE") {
            eprintln!(
                "identifier_store:fold_executed_assignment target={target_name} materialized={materialized:?} resolved_local={:?} local_value={:?} global_value={:?} capture_hidden={:?}",
                self.resolve_current_local_binding(&target_name),
                self.state
                    .speculation
                    .static_semantics
                    .local_value_binding(&target_name),
                self.global_value_binding(&target_name),
                self.resolve_user_function_capture_hidden_name(&target_name),
            );
        }
        is_pure_literal(&materialized).then_some(materialized)
    }

    fn numeric_self_update_value(&self, name: &str, expression: &Expression) -> Option<Expression> {
        let Expression::Binary { op, left, right } = expression else {
            return None;
        };
        let left_is_target =
            matches!(left.as_ref(), Expression::Identifier(source) if source == name);
        let right_is_target =
            matches!(right.as_ref(), Expression::Identifier(source) if source == name);
        if !left_is_target && !right_is_target {
            return None;
        }

        let target_expression = Expression::Identifier(name.to_string());
        let current = self.resolve_static_number_value(&target_expression)?;
        let left_number = if left_is_target {
            current
        } else {
            self.resolve_static_number_value(left)?
        };
        let right_number = if right_is_target {
            current
        } else {
            self.resolve_static_number_value(right)?
        };
        let value = match op {
            BinaryOp::Add => left_number + right_number,
            BinaryOp::Subtract => left_number - right_number,
            BinaryOp::Multiply => left_number * right_number,
            BinaryOp::Divide => left_number / right_number,
            BinaryOp::Modulo => left_number % right_number,
            BinaryOp::Exponentiate => left_number.powf(right_number),
            _ => return None,
        };
        Some(Expression::Number(value))
    }

    fn expression_is_numeric_self_arithmetic_shape(name: &str, expression: &Expression) -> bool {
        match expression {
            Expression::Identifier(source) => source == name,
            Expression::Number(_) => true,
            Expression::Unary {
                op: UnaryOp::Negate,
                expression,
            } => Self::expression_is_numeric_self_arithmetic_shape(name, expression),
            Expression::Binary { op, left, right } => {
                matches!(
                    op,
                    BinaryOp::Add
                        | BinaryOp::Subtract
                        | BinaryOp::Multiply
                        | BinaryOp::Divide
                        | BinaryOp::Modulo
                        | BinaryOp::Exponentiate
                ) && Self::expression_is_numeric_self_arithmetic_shape(name, left)
                    && Self::expression_is_numeric_self_arithmetic_shape(name, right)
            }
            _ => false,
        }
    }

    fn expression_is_self_referential_arithmetic(name: &str, expression: &Expression) -> bool {
        fn arithmetic_expression_mentions_name(name: &str, expression: &Expression) -> bool {
            match expression {
                Expression::Identifier(source) => source == name,
                Expression::Number(_) | Expression::BigInt(_) => false,
                Expression::Unary { expression, .. } => {
                    arithmetic_expression_mentions_name(name, expression)
                }
                Expression::Binary { left, right, .. } => {
                    arithmetic_expression_mentions_name(name, left)
                        || arithmetic_expression_mentions_name(name, right)
                }
                Expression::Member { object, property } => {
                    arithmetic_expression_mentions_name(name, object)
                        || arithmetic_expression_mentions_name(name, property)
                }
                _ => false,
            }
        }

        matches!(
            expression,
            Expression::Binary {
                op: BinaryOp::Add
                    | BinaryOp::Subtract
                    | BinaryOp::Multiply
                    | BinaryOp::Divide
                    | BinaryOp::Modulo
                    | BinaryOp::Exponentiate
                    | BinaryOp::BitwiseAnd
                    | BinaryOp::BitwiseOr
                    | BinaryOp::BitwiseXor
                    | BinaryOp::LeftShift
                    | BinaryOp::RightShift
                    | BinaryOp::UnsignedRightShift,
                ..
            }
        ) && arithmetic_expression_mentions_name(name, expression)
    }

    fn runtime_opaque_identifier_value_store(
        canonical_value_expression: Expression,
        resolved_local_binding: Option<(String, u32)>,
        kind: Option<StaticValueKind>,
    ) -> PreparedIdentifierValueStore {
        PreparedIdentifierValueStore {
            canonical_value_expression: canonical_value_expression.clone(),
            tracked_value_expression: canonical_value_expression.clone(),
            descriptor_binding_expression: Expression::Undefined,
            tracked_object_expression: Expression::Undefined,
            call_source_snapshot_expression: None,
            prototype_source_snapshot_expression: None,
            function_binding_expression: Expression::Undefined,
            function_binding: None,
            object_binding_expression: Expression::Undefined,
            object_binding: None,
            kind: kind.or(Some(StaticValueKind::Unknown)),
            static_string_value: None,
            exact_static_number: None,
            array_binding: None,
            module_assignment_expression: canonical_value_expression,
            resolved_local_binding,
            returned_descriptor_binding: None,
            runtime_value_override: None,
            opaque_runtime_value: true,
        }
    }

    fn self_referential_arithmetic_opaque_kind(
        &self,
        name: &str,
        expression: &Expression,
    ) -> Option<StaticValueKind> {
        fn is_numeric(kind: Option<StaticValueKind>) -> bool {
            matches!(kind, Some(StaticValueKind::Number))
        }

        match expression {
            Expression::Identifier(source) if source == name => self.lookup_identifier_kind(name),
            Expression::Number(_) => Some(StaticValueKind::Number),
            Expression::Unary {
                op: UnaryOp::Negate,
                expression,
            } => is_numeric(self.self_referential_arithmetic_opaque_kind(name, expression))
                .then_some(StaticValueKind::Number),
            Expression::Binary { op, left, right } => {
                if !matches!(
                    op,
                    BinaryOp::Add
                        | BinaryOp::Subtract
                        | BinaryOp::Multiply
                        | BinaryOp::Divide
                        | BinaryOp::Modulo
                        | BinaryOp::Exponentiate
                        | BinaryOp::BitwiseAnd
                        | BinaryOp::BitwiseOr
                        | BinaryOp::BitwiseXor
                        | BinaryOp::LeftShift
                        | BinaryOp::RightShift
                        | BinaryOp::UnsignedRightShift
                ) {
                    return None;
                }
                let left_kind = self.self_referential_arithmetic_operand_kind(name, left);
                let right_kind = self.self_referential_arithmetic_operand_kind(name, right);
                matches!(
                    (left_kind, right_kind),
                    (Some(StaticValueKind::Number), Some(StaticValueKind::Number))
                )
                .then_some(StaticValueKind::Number)
            }
            _ => self.self_referential_arithmetic_leaf_kind(expression),
        }
    }

    fn self_referential_arithmetic_operand_kind(
        &self,
        name: &str,
        expression: &Expression,
    ) -> Option<StaticValueKind> {
        match expression {
            Expression::Binary { .. }
            | Expression::Unary {
                op: UnaryOp::Negate,
                ..
            }
            | Expression::Identifier(_)
            | Expression::Number(_) => self.self_referential_arithmetic_opaque_kind(name, expression),
            _ => self.self_referential_arithmetic_leaf_kind(expression),
        }
    }

    fn self_referential_arithmetic_leaf_kind(
        &self,
        expression: &Expression,
    ) -> Option<StaticValueKind> {
        if self.expression_is_numeric_addition_operand(expression) {
            return Some(StaticValueKind::Number);
        }
        let materialized = self.materialize_static_expression(expression);
        if !static_expression_matches(&materialized, expression)
            && self.expression_is_numeric_addition_operand(&materialized)
        {
            return Some(StaticValueKind::Number);
        }
        None
    }

    fn primitive_identifier_value_store(
        value: Expression,
        resolved_local_binding: Option<(String, u32)>,
    ) -> PreparedIdentifierValueStore {
        let kind = match &value {
            Expression::Number(_) => StaticValueKind::Number,
            Expression::BigInt(_) => StaticValueKind::BigInt,
            Expression::String(_) => StaticValueKind::String,
            Expression::Bool(_) => StaticValueKind::Bool,
            Expression::Null => StaticValueKind::Null,
            Expression::Undefined => StaticValueKind::Undefined,
            _ => StaticValueKind::Unknown,
        };
        let static_string_value = match &value {
            Expression::String(text) => Some(text.clone()),
            _ => None,
        };
        PreparedIdentifierValueStore {
            canonical_value_expression: value.clone(),
            tracked_value_expression: value.clone(),
            descriptor_binding_expression: value.clone(),
            tracked_object_expression: Expression::Undefined,
            call_source_snapshot_expression: None,
            prototype_source_snapshot_expression: None,
            function_binding_expression: Expression::Undefined,
            function_binding: None,
            object_binding_expression: Expression::Undefined,
            object_binding: None,
            kind: Some(kind),
            static_string_value,
            exact_static_number: None,
            array_binding: None,
            module_assignment_expression: value,
            resolved_local_binding,
            returned_descriptor_binding: None,
            runtime_value_override: None,
            opaque_runtime_value: false,
        }
    }

    fn number_identifier_value_store(
        value: Expression,
        resolved_local_binding: Option<(String, u32)>,
    ) -> PreparedIdentifierValueStore {
        Self::primitive_identifier_value_store(value, resolved_local_binding)
    }

    fn static_call_result_object_literal_store_is_safe(
        &self,
        canonical_value_expression: &Expression,
        tracked_value_expression: &Expression,
    ) -> bool {
        let Expression::Call { callee, arguments } = canonical_value_expression else {
            return false;
        };
        if !matches!(tracked_value_expression, Expression::Object(_)) {
            return false;
        }
        if arguments.iter().any(|argument| match argument {
            CallArgument::Expression(expression) => {
                !inline_summary_side_effect_free_expression(expression)
            }
            CallArgument::Spread(_) => true,
        }) {
            return false;
        }
        let Some(user_function) = self.resolve_user_function_from_expression(callee) else {
            return false;
        };
        if user_function.is_async()
            || user_function.is_generator()
            || user_function.has_parameter_defaults()
            || user_function.has_lowered_pattern_parameters()
            || self.user_function_uses_direct_arguments_object(user_function)
            || self.user_function_mentions_private_member_access(user_function)
            || self.user_function_mentions_direct_eval(user_function)
        {
            return false;
        }
        self.prepared_user_function_assigned_nonlocal_bindings(user_function)
            .is_empty()
            && self
                .collect_user_function_updated_nonlocal_bindings(user_function)
                .is_empty()
    }

    pub(super) fn prepare_identifier_value_store(
        &mut self,
        name: &str,
        value_expression: &Expression,
    ) -> PreparedIdentifierValueStore {
        let trace_identifier_store = crate::ayy_env_flag!("AYY_TRACE_IDENTIFIER_STORE");
        let trace_prepare_timing = crate::ayy_env_flag!("AYY_TRACE_IDENTIFIER_PREPARE_TIMING");
        let timing_start = trace_prepare_timing.then(std::time::Instant::now);
        let mut timing_last = timing_start;
        let mut trace_timing = |label: &str| {
            if let Some(previous) = timing_last {
                let now = std::time::Instant::now();
                let total_ms = timing_start
                    .map(|start| now.duration_since(start).as_millis())
                    .unwrap_or(0);
                eprintln!(
                    "identifier_prepare_timing name={name} step={label} elapsed_ms={} total_ms={total_ms}",
                    now.duration_since(previous).as_millis()
                );
                timing_last = Some(now);
            }
        };
        if trace_identifier_store {
            eprintln!("identifier_store:{name}:prepare:start");
            if std::env::var_os("AYY_TRACE_IDENTIFIER_STORE_BACKTRACE").is_some() {
                eprintln!(
                    "identifier_store:{name}:prepare:backtrace\n{}",
                    std::backtrace::Backtrace::force_capture()
                );
            }
        }
        let is_for_in_keys_temp = name.starts_with("__ayy_for_in_keys_");
        let private_brand_initializer =
            self.is_private_brand_binding_store_initializer(name, value_expression);
        let resolved_local_binding = self.resolve_current_local_binding(name);
        trace_timing("resolve_local");
        if private_brand_initializer || is_for_in_keys_temp {
            let tracked_value_expression = Expression::Undefined;
            return PreparedIdentifierValueStore {
                canonical_value_expression: value_expression.clone(),
                tracked_value_expression: tracked_value_expression.clone(),
                descriptor_binding_expression: tracked_value_expression.clone(),
                tracked_object_expression: Expression::Undefined,
                call_source_snapshot_expression: None,
                prototype_source_snapshot_expression: None,
                function_binding_expression: tracked_value_expression.clone(),
                function_binding: None,
                object_binding_expression: Expression::Undefined,
                object_binding: None,
                kind: is_for_in_keys_temp.then_some(StaticValueKind::Object),
                static_string_value: None,
                exact_static_number: None,
                array_binding: if is_for_in_keys_temp {
                    self.resolve_array_binding_from_expression(value_expression)
                } else {
                    None
                },
                module_assignment_expression: tracked_value_expression,
                resolved_local_binding,
                returned_descriptor_binding: None,
                runtime_value_override: None,
                opaque_runtime_value: false,
            };
        }
        let with_scoped_value_expression = if let Expression::Identifier(value_name) =
            value_expression
            && let Some(scope_object) =
                self.resolve_with_scope_binding_for_specialization(value_name)
        {
            self.materialize_static_expression(&Expression::Member {
                object: Box::new(scope_object),
                property: Box::new(Expression::String(value_name.clone())),
            })
        } else {
            value_expression.clone()
        };
        let mut canonical_value_expression = if context_expression_references_internal_iterator_step(
            &with_scoped_value_expression,
        ) {
            with_scoped_value_expression.clone()
        } else {
            self.prepare_special_assignment_expression(&with_scoped_value_expression)
                .unwrap_or_else(|| with_scoped_value_expression.clone())
        };
        if let Some(static_iterator_step_value) =
            self.resolve_static_iterator_step_assignment_value(&canonical_value_expression)
        {
            canonical_value_expression = static_iterator_step_value;
        }
        if let Some(folded) = self.fold_executed_assignment_store_value(&canonical_value_expression)
        {
            canonical_value_expression = folded;
        }
        trace_timing("canonical");
        if Self::expression_is_numeric_self_arithmetic_shape(name, &canonical_value_expression)
            && let Some(number) = self.resolve_static_number_value(&canonical_value_expression)
        {
            canonical_value_expression = Expression::Number(number);
        }
        if let Some(folded) = self.numeric_self_update_value(name, &canonical_value_expression) {
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:numeric_self_update {folded:?}");
            }
            return Self::number_identifier_value_store(folded, resolved_local_binding);
        }
        if let Some(value) = self.resolve_simple_array_append_return_argument_static_call_value(
            &canonical_value_expression,
        ) {
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:simple_array_append_return_value {value:?}");
            }
            return Self::primitive_identifier_value_store(value, resolved_local_binding);
        }
        if Self::expression_is_self_referential_arithmetic(name, &canonical_value_expression) {
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:opaque_self_referential_arithmetic");
            }
            let kind =
                self.self_referential_arithmetic_opaque_kind(name, &canonical_value_expression);
            return Self::runtime_opaque_identifier_value_store(
                canonical_value_expression,
                resolved_local_binding,
                kind,
            );
        }
        let active_loop_string_assignment = self
            .active_loop_string_assignment_snapshot(&canonical_value_expression)
            .is_some();
        if active_loop_string_assignment {
            self.record_active_loop_eval_source_alias(name, &canonical_value_expression);
            return PreparedIdentifierValueStore {
                canonical_value_expression: canonical_value_expression.clone(),
                tracked_value_expression: Expression::Undefined,
                descriptor_binding_expression: Expression::Undefined,
                tracked_object_expression: Expression::Undefined,
                call_source_snapshot_expression: None,
                prototype_source_snapshot_expression: None,
                function_binding_expression: Expression::Undefined,
                function_binding: None,
                object_binding_expression: Expression::Undefined,
                object_binding: None,
                kind: Some(StaticValueKind::String),
                static_string_value: None,
                exact_static_number: None,
                array_binding: None,
                module_assignment_expression: canonical_value_expression.clone(),
                resolved_local_binding,
                returned_descriptor_binding: None,
                runtime_value_override: None,
                opaque_runtime_value: false,
            };
        }
        if self.expression_depends_on_active_loop_assignment(&canonical_value_expression)
            && Self::expression_is_string_from_char_code_call(&canonical_value_expression)
        {
            self.record_active_loop_eval_source_alias(name, &canonical_value_expression);
        }
        if expression_is_object_create_null_call(&canonical_value_expression) {
            let object_expression = Expression::Object(Vec::new());
            return PreparedIdentifierValueStore {
                canonical_value_expression: canonical_value_expression.clone(),
                tracked_value_expression: object_expression.clone(),
                descriptor_binding_expression: Expression::Undefined,
                tracked_object_expression: object_expression.clone(),
                call_source_snapshot_expression: None,
                prototype_source_snapshot_expression: Some(Expression::Null),
                function_binding_expression: Expression::Undefined,
                function_binding: None,
                object_binding_expression: object_expression,
                object_binding: Some(empty_object_value_binding()),
                kind: Some(StaticValueKind::Object),
                static_string_value: None,
                exact_static_number: None,
                array_binding: None,
                module_assignment_expression: canonical_value_expression.clone(),
                resolved_local_binding,
                returned_descriptor_binding: None,
                runtime_value_override: None,
                opaque_runtime_value: false,
            };
        }
        if trace_identifier_store {
            eprintln!("identifier_store:{name}:canonical {canonical_value_expression:?}");
        }
        if let Some(store) = self.static_array_literal_identifier_value_store(
            &canonical_value_expression,
            resolved_local_binding.clone(),
        ) {
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:static_array_literal_fast_path");
            }
            return store;
        }
        if let Some(store) = self.static_object_literal_identifier_value_store(
            &canonical_value_expression,
            resolved_local_binding.clone(),
        ) {
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:static_object_literal_fast_path");
            }
            return store;
        }
        if expression_is_dynamic_module_namespace_descriptor_call(self, &canonical_value_expression)
        {
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:dynamic_module_namespace_descriptor");
            }
            return PreparedIdentifierValueStore {
                canonical_value_expression: canonical_value_expression.clone(),
                tracked_value_expression: canonical_value_expression.clone(),
                descriptor_binding_expression: Expression::Undefined,
                tracked_object_expression: Expression::Undefined,
                call_source_snapshot_expression: None,
                prototype_source_snapshot_expression: None,
                function_binding_expression: Expression::Undefined,
                function_binding: None,
                object_binding_expression: Expression::Undefined,
                object_binding: None,
                kind: Some(StaticValueKind::Unknown),
                static_string_value: None,
                exact_static_number: None,
                array_binding: None,
                module_assignment_expression: canonical_value_expression,
                resolved_local_binding,
                returned_descriptor_binding: None,
                runtime_value_override: None,
                opaque_runtime_value: false,
            };
        }
        if self.identifier_store_is_unresolvable_self_reference(name, &canonical_value_expression) {
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:unresolvable_self_reference_fast_path");
            }
            return PreparedIdentifierValueStore {
                canonical_value_expression: canonical_value_expression.clone(),
                tracked_value_expression: canonical_value_expression.clone(),
                descriptor_binding_expression: Expression::Undefined,
                tracked_object_expression: Expression::Undefined,
                call_source_snapshot_expression: None,
                prototype_source_snapshot_expression: None,
                function_binding_expression: Expression::Undefined,
                function_binding: None,
                object_binding_expression: Expression::Undefined,
                object_binding: None,
                kind: Some(StaticValueKind::Unknown),
                static_string_value: None,
                exact_static_number: None,
                array_binding: None,
                module_assignment_expression: canonical_value_expression,
                resolved_local_binding,
                returned_descriptor_binding: None,
                runtime_value_override: None,
                opaque_runtime_value: true,
            };
        }
        if expression_is_nested_assert_helper_runtime_value(&canonical_value_expression) {
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:nested_assert_helper_runtime_fast_path");
            }
            return PreparedIdentifierValueStore {
                canonical_value_expression: canonical_value_expression.clone(),
                tracked_value_expression: canonical_value_expression.clone(),
                descriptor_binding_expression: Expression::Undefined,
                tracked_object_expression: Expression::Undefined,
                call_source_snapshot_expression: None,
                prototype_source_snapshot_expression: None,
                function_binding_expression: Expression::Undefined,
                function_binding: None,
                object_binding_expression: Expression::Undefined,
                object_binding: None,
                kind: Some(StaticValueKind::Unknown),
                static_string_value: None,
                exact_static_number: None,
                array_binding: None,
                module_assignment_expression: canonical_value_expression,
                resolved_local_binding,
                returned_descriptor_binding: None,
                runtime_value_override: None,
                opaque_runtime_value: true,
            };
        }
        if let Expression::Member { object, property } = &canonical_value_expression
            && matches!(property.as_ref(), Expression::String(property_name) if property_name == "constructor")
            && let Some(binding) = self.resolve_function_binding_from_expression(object)
        {
            let constructor_name = match binding {
                LocalFunctionBinding::User(function_name) => self
                    .user_function(&function_name)
                    .map(|function| match function.kind {
                        FunctionKind::Ordinary => "Function",
                        FunctionKind::Generator => "GeneratorFunction",
                        FunctionKind::Async => "AsyncFunction",
                        FunctionKind::AsyncGenerator => "AsyncGeneratorFunction",
                    })
                    .unwrap_or("Function"),
                LocalFunctionBinding::Builtin(_) => "Function",
            };
            if is_function_constructor_builtin(constructor_name) {
                let materialized_constructor = Expression::Identifier(constructor_name.to_string());
                if trace_identifier_store {
                    eprintln!(
                        "identifier_store:{name}:function_constructor_alias {constructor_name}"
                    );
                }
                return PreparedIdentifierValueStore {
                    canonical_value_expression: canonical_value_expression.clone(),
                    tracked_value_expression: materialized_constructor.clone(),
                    descriptor_binding_expression: Expression::Undefined,
                    tracked_object_expression: Expression::Undefined,
                    call_source_snapshot_expression: None,
                    prototype_source_snapshot_expression: None,
                    function_binding_expression: materialized_constructor.clone(),
                    function_binding: Some(LocalFunctionBinding::Builtin(
                        constructor_name.to_string(),
                    )),
                    object_binding_expression: Expression::Undefined,
                    object_binding: None,
                    kind: Some(StaticValueKind::Function),
                    static_string_value: None,
                    exact_static_number: None,
                    array_binding: None,
                    module_assignment_expression: materialized_constructor,
                    resolved_local_binding,
                    returned_descriptor_binding: None,
                    runtime_value_override: None,
                    opaque_runtime_value: false,
                };
            }
        }
        if let Expression::Member { object, property } = &canonical_value_expression
            && matches!(object.as_ref(), Expression::Call { .. })
            && let Some(snapshot) = self
                .state
                .speculation
                .static_semantics
                .last_bound_user_function_call
                .as_ref()
            && snapshot
                .source_expression
                .as_ref()
                .is_some_and(|source| static_expression_matches(source, object))
            && let Some(result_expression) = snapshot.result_expression.as_ref()
        {
            let resolved_property = self
                .resolve_property_key_expression(property)
                .unwrap_or_else(|| self.materialize_static_expression(property));
            let snapshot_result_binding =
                self.resolve_object_binding_from_expression(result_expression);
            let member_value_expression = snapshot_result_binding
                .as_ref()
                .and_then(|binding| object_binding_lookup_value(binding, &resolved_property))
                .cloned()
                .or_else(|| {
                    matches!(
                        resolved_property,
                        Expression::String(_) | Expression::Number(_)
                    )
                    .then_some(Expression::Undefined)
                });
            if let Some(member_value_expression) = member_value_expression {
                let function_binding_expression = self
                    .resolve_static_function_binding_store_expression_with_context(
                        &member_value_expression,
                        Some(snapshot.function_name.as_str()),
                    );
                let function_binding = self
                    .resolve_function_binding_from_expression_with_context(
                        &function_binding_expression,
                        Some(snapshot.function_name.as_str()),
                    )
                    .or_else(|| {
                        self.resolve_function_binding_from_expression(&function_binding_expression)
                    });
                let object_binding =
                    self.resolve_object_binding_from_expression(&member_value_expression);
                let kind = self
                    .infer_value_kind(&member_value_expression)
                    .or_else(|| object_binding.as_ref().map(|_| StaticValueKind::Object))
                    .unwrap_or(StaticValueKind::Unknown);
                let static_string_value = (kind == StaticValueKind::String)
                    .then(|| self.resolve_static_string_value(&member_value_expression))
                    .flatten();
                let exact_static_number = self
                    .resolve_static_number_value(&member_value_expression)
                    .filter(|number| {
                        number.is_nan()
                            || !number.is_finite()
                            || number.fract() != 0.0
                            || (*number == 0.0 && number.is_sign_negative())
                    });
                let array_binding =
                    self.resolve_array_binding_from_expression(&member_value_expression);
                let module_assignment_expression =
                    self.materialize_static_expression(&member_value_expression);
                if trace_identifier_store {
                    eprintln!(
                        "identifier_store:{name}:call_snapshot_member value={member_value_expression:?}"
                    );
                }
                return PreparedIdentifierValueStore {
                    canonical_value_expression: canonical_value_expression.clone(),
                    tracked_value_expression: member_value_expression.clone(),
                    descriptor_binding_expression: member_value_expression.clone(),
                    tracked_object_expression: member_value_expression.clone(),
                    call_source_snapshot_expression: snapshot.source_expression.clone(),
                    prototype_source_snapshot_expression: None,
                    function_binding_expression,
                    function_binding,
                    object_binding_expression: member_value_expression,
                    object_binding,
                    kind: Some(kind),
                    static_string_value,
                    exact_static_number,
                    array_binding,
                    module_assignment_expression,
                    resolved_local_binding,
                    returned_descriptor_binding: None,
                    runtime_value_override: None,
                    opaque_runtime_value: false,
                };
            }
        }
        if self.is_direct_local_array_iterator_method_call_expression(&canonical_value_expression) {
            let matched_call_snapshot = self
                .state
                .speculation
                .static_semantics
                .last_bound_user_function_call
                .as_ref()
                .and_then(|snapshot| {
                    let source_expression = snapshot.source_expression.as_ref()?;
                    static_expression_matches(source_expression, &canonical_value_expression)
                        .then_some(snapshot)
                });
            let call_result_snapshot_expression = matched_call_snapshot
                .and_then(|snapshot| snapshot.result_expression.as_ref())
                .map(|result| match result {
                    Expression::Identifier(_) | Expression::This => result.clone(),
                    _ => self.materialize_static_expression(result),
                });
            let metadata_value_expression = call_result_snapshot_expression
                .as_ref()
                .unwrap_or(&canonical_value_expression);
            let object_binding_expression = call_result_snapshot_expression
                .clone()
                .unwrap_or(Expression::Undefined);
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:direct_iterator_method_call");
            }
            if trace_identifier_store {
                eprintln!(
                    "identifier_store:{name}:direct_iterator_method_call:object_binding:start"
                );
            }
            let object_binding =
                self.resolve_object_binding_from_expression(&object_binding_expression);
            if trace_identifier_store {
                eprintln!(
                    "identifier_store:{name}:direct_iterator_method_call:object_binding:done"
                );
                eprintln!("identifier_store:{name}:direct_iterator_method_call:kind:start");
            }
            let kind = self
                .infer_value_kind(metadata_value_expression)
                .or(Some(StaticValueKind::Object));
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:direct_iterator_method_call:kind:done");
                eprintln!("identifier_store:{name}:direct_iterator_method_call:array:start");
            }
            let array_binding = None;
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:direct_iterator_method_call:array:done");
            }
            return PreparedIdentifierValueStore {
                canonical_value_expression: canonical_value_expression.clone(),
                tracked_value_expression: canonical_value_expression.clone(),
                descriptor_binding_expression: object_binding_expression.clone(),
                tracked_object_expression: object_binding_expression.clone(),
                call_source_snapshot_expression: matched_call_snapshot
                    .and_then(|snapshot| snapshot.source_expression.as_ref().cloned()),
                prototype_source_snapshot_expression: None,
                function_binding_expression: Expression::Undefined,
                function_binding: None,
                object_binding,
                object_binding_expression,
                kind,
                static_string_value: None,
                exact_static_number: None,
                array_binding,
                module_assignment_expression: metadata_value_expression.clone(),
                resolved_local_binding,
                returned_descriptor_binding: None,
                runtime_value_override: None,
                opaque_runtime_value: false,
            };
        }
        let iterator_step_member_kind = if let Expression::Member { object, property } =
            &canonical_value_expression
            && let Expression::String(property_name) = property.as_ref()
            && (property_name == "done" || property_name == "value")
            && self
                .resolve_iterator_step_binding_from_expression(object)
                .is_some()
        {
            Some(if property_name == "done" {
                StaticValueKind::Bool
            } else {
                StaticValueKind::Unknown
            })
        } else {
            None
        };
        if let Some(kind) = iterator_step_member_kind {
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:iterator_step_member");
            }
            return PreparedIdentifierValueStore {
                canonical_value_expression: canonical_value_expression.clone(),
                tracked_value_expression: canonical_value_expression.clone(),
                descriptor_binding_expression: Expression::Undefined,
                tracked_object_expression: Expression::Undefined,
                call_source_snapshot_expression: None,
                prototype_source_snapshot_expression: None,
                function_binding_expression: Expression::Undefined,
                function_binding: None,
                object_binding_expression: Expression::Undefined,
                object_binding: None,
                kind: Some(kind),
                static_string_value: None,
                exact_static_number: None,
                array_binding: None,
                module_assignment_expression: canonical_value_expression.clone(),
                resolved_local_binding,
                returned_descriptor_binding: None,
                runtime_value_override: None,
                opaque_runtime_value: false,
            };
        }
        let local_array_iterator_next_call =
            self.is_local_array_iterator_next_call_expression(&canonical_value_expression);
        let local_simple_async_generator_next_call =
            self.is_local_simple_async_generator_next_call_expression(&canonical_value_expression);
        let internal_iterator_step_next_call = (name.starts_with("__ayy_array_step_")
            || name.starts_with("__ayy_for_of_step_"))
            && matches!(
                &canonical_value_expression,
                Expression::Call { callee, arguments }
                    if arguments.is_empty()
                        && matches!(
                            callee.as_ref(),
                            Expression::Member { property, .. }
                                if matches!(
                                    property.as_ref(),
                                    Expression::String(property_name) if property_name == "next"
                                )
                        )
            );
        if expression_is_static_promise_then_call(&canonical_value_expression) {
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:promise_then_call");
            }
            return PreparedIdentifierValueStore {
                canonical_value_expression: canonical_value_expression.clone(),
                tracked_value_expression: canonical_value_expression.clone(),
                descriptor_binding_expression: Expression::Undefined,
                tracked_object_expression: Expression::Undefined,
                call_source_snapshot_expression: None,
                prototype_source_snapshot_expression: None,
                function_binding_expression: Expression::Undefined,
                function_binding: None,
                object_binding_expression: Expression::Undefined,
                object_binding: None,
                kind: Some(StaticValueKind::Object),
                static_string_value: None,
                exact_static_number: None,
                array_binding: None,
                module_assignment_expression: canonical_value_expression,
                resolved_local_binding,
                returned_descriptor_binding: None,
                runtime_value_override: None,
                opaque_runtime_value: false,
            };
        }
        let mut tracked_value_expression = match &canonical_value_expression {
            Expression::Call { callee, arguments } => {
                let preserve_canonical_call_expression = local_array_iterator_next_call
                    || local_simple_async_generator_next_call
                    || internal_iterator_step_next_call
                    || self
                        .resolve_user_function_from_expression(callee)
                        .is_some_and(|user_function| user_function.is_async())
                    || (self.call_callee_may_resolve_simple_generator_source(callee)
                        && self
                            .resolve_simple_generator_source(&canonical_value_expression)
                            .is_some())
                    || self
                        .resolve_async_yield_delegate_generator_plan(
                            &canonical_value_expression,
                            "__ayy_async_delegate_completion",
                        )
                        .is_some();
                if preserve_canonical_call_expression {
                    canonical_value_expression.clone()
                } else {
                    self.resolve_static_call_result_expression_with_context(
                        callee,
                        arguments,
                        self.current_function_name(),
                    )
                    .map(|(value, function_name)| {
                        let normalized = self.normalize_static_call_result_after_runtime_snapshot(
                            value,
                            function_name,
                        );
                        self.normalize_direct_function_expression_call_result_in_with_scope(
                            callee, normalized,
                        )
                    })
                    .unwrap_or_else(|| canonical_value_expression.clone())
                }
            }
            Expression::Member { object, property } => {
                if self
                    .resolve_member_function_capture_slots(object, property)
                    .is_some()
                {
                    canonical_value_expression.clone()
                } else if matches!(
                    object.as_ref(),
                    Expression::Identifier(name) if name.starts_with("__ayy_inline_param_")
                ) && let Some(value) =
                    self.resolve_static_effect_member_value(&canonical_value_expression)
                {
                    value
                } else {
                    self.resolve_member_getter_binding(object, property)
                        .and_then(|binding| {
                            self.resolve_function_binding_static_return_expression_with_call_frame(
                                &binding,
                                &[],
                                object,
                            )
                        })
                        .unwrap_or_else(|| canonical_value_expression.clone())
                }
            }
            _ => canonical_value_expression.clone(),
        };
        trace_timing("tracked");
        if let Some(length_snapshot) =
            self.static_array_length_store_snapshot(&tracked_value_expression)
        {
            tracked_value_expression = length_snapshot;
        }
        trace_timing("array_length_snapshot");
        if self.static_call_result_object_literal_store_is_safe(
            &canonical_value_expression,
            &tracked_value_expression,
        ) && let Some(store) = self.static_object_literal_identifier_value_store(
            &tracked_value_expression,
            resolved_local_binding.clone(),
        ) {
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:static_call_object_literal_fast_path");
            }
            trace_timing("static_call_object_literal_fast_path");
            return store;
        }
        if trace_identifier_store {
            eprintln!("identifier_store:{name}:tracked {tracked_value_expression:?}");
        }
        if let Some(kind) = async_delegate_result_member_kind(&canonical_value_expression) {
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:async_delegate_result_member");
            }
            return PreparedIdentifierValueStore {
                canonical_value_expression: canonical_value_expression.clone(),
                tracked_value_expression: tracked_value_expression.clone(),
                descriptor_binding_expression: Expression::Undefined,
                tracked_object_expression: Expression::Undefined,
                call_source_snapshot_expression: None,
                prototype_source_snapshot_expression: None,
                function_binding_expression: Expression::Undefined,
                function_binding: None,
                object_binding_expression: Expression::Undefined,
                object_binding: None,
                kind: Some(kind),
                static_string_value: None,
                exact_static_number: None,
                array_binding: None,
                module_assignment_expression: canonical_value_expression.clone(),
                resolved_local_binding,
                returned_descriptor_binding: None,
                runtime_value_override: None,
                opaque_runtime_value: true,
            };
        }
        if static_expression_matches(&tracked_value_expression, &canonical_value_expression)
            && expression_is_non_prototype_nested_member(&canonical_value_expression)
        {
            let kind = self
                .snapshot_effectful_member_read_for_static_store(&canonical_value_expression)
                .and_then(|value| self.infer_value_kind(&value))
                .unwrap_or(StaticValueKind::Unknown);
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:nested_member_fast_path kind={kind:?}");
            }
            return PreparedIdentifierValueStore {
                canonical_value_expression: canonical_value_expression.clone(),
                tracked_value_expression: tracked_value_expression.clone(),
                descriptor_binding_expression: Expression::Undefined,
                tracked_object_expression: Expression::Undefined,
                call_source_snapshot_expression: None,
                prototype_source_snapshot_expression: None,
                function_binding_expression: Expression::Undefined,
                function_binding: None,
                object_binding_expression: Expression::Undefined,
                object_binding: None,
                kind: Some(kind),
                static_string_value: None,
                exact_static_number: None,
                array_binding: None,
                module_assignment_expression: canonical_value_expression.clone(),
                resolved_local_binding,
                returned_descriptor_binding: None,
                runtime_value_override: None,
                opaque_runtime_value: true,
            };
        }
        if expression_is_promise_all_call(&canonical_value_expression) {
            let resolved_promise = promise_resolve_array_placeholder();
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:promise_all_static_placeholder");
            }
            return PreparedIdentifierValueStore {
                canonical_value_expression: canonical_value_expression.clone(),
                tracked_value_expression: resolved_promise.clone(),
                descriptor_binding_expression: Expression::Undefined,
                tracked_object_expression: Expression::Undefined,
                call_source_snapshot_expression: None,
                prototype_source_snapshot_expression: None,
                function_binding_expression: Expression::Undefined,
                function_binding: None,
                object_binding_expression: Expression::Undefined,
                object_binding: None,
                kind: Some(StaticValueKind::Object),
                static_string_value: None,
                exact_static_number: None,
                array_binding: None,
                module_assignment_expression: resolved_promise,
                resolved_local_binding,
                returned_descriptor_binding: None,
                runtime_value_override: None,
                opaque_runtime_value: false,
            };
        }
        if local_array_iterator_next_call || internal_iterator_step_next_call {
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:local_iterator_next");
            }
            return PreparedIdentifierValueStore {
                canonical_value_expression: canonical_value_expression.clone(),
                tracked_value_expression: tracked_value_expression.clone(),
                descriptor_binding_expression: Expression::Undefined,
                tracked_object_expression: Expression::Undefined,
                call_source_snapshot_expression: None,
                prototype_source_snapshot_expression: None,
                function_binding_expression: Expression::Undefined,
                function_binding: None,
                object_binding_expression: Expression::Undefined,
                object_binding: None,
                kind: Some(StaticValueKind::Object),
                static_string_value: None,
                exact_static_number: None,
                array_binding: None,
                module_assignment_expression: canonical_value_expression.clone(),
                resolved_local_binding,
                returned_descriptor_binding: None,
                runtime_value_override: None,
                opaque_runtime_value: false,
            };
        }
        if context_expression_references_internal_iterator_step(&canonical_value_expression)
            || context_expression_references_internal_iterator_step(&tracked_value_expression)
        {
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:iterator_step_value");
            }
            // Awaiting a statically known non-thenable iterator element is an
            // identity step; track the inner member read so for-await values
            // keep the same metadata as sync for-of.
            let canonical_value_expression = self
                .for_await_step_value_without_await(&canonical_value_expression)
                .unwrap_or_else(|| canonical_value_expression.clone());
            let tracked_value_expression = self
                .for_await_step_value_without_await(&tracked_value_expression)
                .unwrap_or_else(|| tracked_value_expression.clone());
            return PreparedIdentifierValueStore {
                canonical_value_expression: canonical_value_expression.clone(),
                tracked_value_expression: tracked_value_expression.clone(),
                descriptor_binding_expression: Expression::Undefined,
                tracked_object_expression: Expression::Undefined,
                call_source_snapshot_expression: None,
                prototype_source_snapshot_expression: None,
                function_binding_expression: Expression::Undefined,
                function_binding: None,
                object_binding_expression: Expression::Undefined,
                object_binding: None,
                kind: None,
                static_string_value: None,
                exact_static_number: None,
                array_binding: None,
                module_assignment_expression: canonical_value_expression.clone(),
                resolved_local_binding,
                returned_descriptor_binding: None,
                runtime_value_override: None,
                opaque_runtime_value: false,
            };
        }
        if expression_is_static_promise_with_resolvers_record(&tracked_value_expression) {
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:promise_with_resolvers_static_record");
            }
            return PreparedIdentifierValueStore {
                canonical_value_expression: canonical_value_expression.clone(),
                tracked_value_expression: tracked_value_expression.clone(),
                descriptor_binding_expression: tracked_value_expression.clone(),
                tracked_object_expression: tracked_value_expression.clone(),
                call_source_snapshot_expression: None,
                prototype_source_snapshot_expression: None,
                function_binding_expression: Expression::Undefined,
                function_binding: None,
                object_binding_expression: tracked_value_expression.clone(),
                object_binding: Some(static_promise_with_resolvers_object_binding()),
                kind: Some(StaticValueKind::Object),
                static_string_value: None,
                exact_static_number: None,
                array_binding: None,
                module_assignment_expression: tracked_value_expression,
                resolved_local_binding,
                returned_descriptor_binding: None,
                runtime_value_override: None,
                opaque_runtime_value: false,
            };
        }
        if !matches!(tracked_value_expression, Expression::Object(_))
            && let Some(function_binding) =
                self.resolve_function_binding_from_expression(&tracked_value_expression)
            && matches!(
                &function_binding,
                LocalFunctionBinding::Builtin(function_name)
                    if parse_bound_function_prototype_call_builtin_name(function_name).is_some()
            )
        {
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:bound_call_builtin_fast_path");
            }
            return PreparedIdentifierValueStore {
                canonical_value_expression: canonical_value_expression.clone(),
                tracked_value_expression: tracked_value_expression.clone(),
                descriptor_binding_expression: Expression::Undefined,
                tracked_object_expression: Expression::Undefined,
                call_source_snapshot_expression: None,
                prototype_source_snapshot_expression: None,
                function_binding_expression: tracked_value_expression.clone(),
                function_binding: Some(function_binding),
                object_binding_expression: Expression::Undefined,
                object_binding: None,
                kind: Some(StaticValueKind::Function),
                static_string_value: None,
                exact_static_number: None,
                array_binding: None,
                module_assignment_expression: tracked_value_expression.clone(),
                resolved_local_binding,
                returned_descriptor_binding: None,
                runtime_value_override: None,
                opaque_runtime_value: false,
            };
        }
        let resolved_descriptor_binding =
            self.resolve_descriptor_binding_from_expression(&canonical_value_expression);
        trace_timing("descriptor_binding");
        if trace_identifier_store {
            eprintln!("identifier_store:{name}:descriptor");
        }
        let returned_descriptor_binding = match &canonical_value_expression {
            Expression::Call { callee, arguments } => self
                .resolve_function_binding_from_expression(callee)
                .and_then(|binding| match binding {
                    LocalFunctionBinding::User(function_name) => self
                        .resolve_static_returned_descriptor_binding_from_user_function_call(
                            &function_name,
                            arguments,
                        ),
                    LocalFunctionBinding::Builtin(_) => None,
                }),
            _ => None,
        };
        trace_timing("returned_descriptor_binding");
        let descriptor_binding_expression = if resolved_descriptor_binding.is_some() {
            canonical_value_expression.clone()
        } else {
            tracked_value_expression.clone()
        };
        let tracked_object_expression = resolved_descriptor_binding
            .as_ref()
            .map(|descriptor| {
                object_binding_to_expression(
                    &self.object_binding_from_property_descriptor(descriptor),
                )
            })
            .unwrap_or_else(|| tracked_value_expression.clone());
        trace_timing("tracked_object_expression");
        let matched_call_snapshot = matches!(
            canonical_value_expression,
            Expression::Call { .. } | Expression::New { .. }
        )
        .then(|| {
            self.state
                .speculation
                .static_semantics
                .last_bound_user_function_call
                .as_ref()
                .and_then(|snapshot| {
                    let source_expression = snapshot.source_expression.as_ref()?;
                    let materialized_source = self.materialize_static_expression(source_expression);
                    let materialized_value =
                        self.materialize_static_expression(&canonical_value_expression);
                    static_expression_matches(&materialized_source, &materialized_value)
                        .then_some(snapshot)
                })
        })
        .flatten();
        trace_timing("matched_call_snapshot");
        let snapshot_is_async_function_call = matched_call_snapshot.is_some_and(|snapshot| {
            self.user_function(&snapshot.function_name)
                .is_some_and(|function| function.is_async() && !function.is_generator())
        });
        let call_result_snapshot_expression = matched_call_snapshot.and_then(|snapshot| {
            if snapshot_is_async_function_call {
                return None;
            }
            snapshot
                .result_expression
                .as_ref()
                .map(|result| match result {
                    Expression::Identifier(_) | Expression::This => result.clone(),
                    _ => self.materialize_static_expression(result),
                })
                .map(|result| {
                    self.replace_call_snapshot_updated_values_with_runtime_reads(
                        &result,
                        &snapshot.updated_bindings,
                    )
                })
        });
        trace_timing("call_result_snapshot");
        let call_snapshot_function_context =
            matched_call_snapshot.map(|snapshot| snapshot.function_name.as_str());
        let call_source_snapshot_expression =
            matched_call_snapshot.and_then(|snapshot| snapshot.source_expression.as_ref().cloned());
        let prototype_source_snapshot_expression = matched_call_snapshot.and_then(|snapshot| {
            snapshot
                .prototype_source_expression
                .as_ref()
                .map(|prototype_source| match prototype_source {
                    Expression::Identifier(_) | Expression::This => prototype_source.clone(),
                    _ => self.materialize_static_expression(prototype_source),
                })
        });
        if trace_identifier_store {
            eprintln!("identifier_store:{name}:call_snapshot");
        }
        let call_result_function_binding =
            call_result_snapshot_expression
                .as_ref()
                .and_then(|expression| {
                    self.resolve_function_binding_from_expression_with_context(
                        expression,
                        call_snapshot_function_context,
                    )
                    .or_else(|| self.resolve_function_binding_from_expression(expression))
                });
        trace_timing("call_result_function_binding");
        let call_result_has_function_binding = call_result_function_binding.is_some();
        let value_is_object_literal = matches!(canonical_value_expression, Expression::Object(_));
        let raw_function_binding_expression = if value_is_object_literal
            || (local_simple_async_generator_next_call && call_result_function_binding.is_none())
        {
            Expression::Undefined
        } else {
            call_result_snapshot_expression
                .as_ref()
                .filter(|_| call_result_function_binding.is_some())
                .unwrap_or(&tracked_value_expression)
                .clone()
        };
        let resolved_function_binding_expression =
            if local_simple_async_generator_next_call && call_result_function_binding.is_none() {
                raw_function_binding_expression.clone()
            } else {
                self.resolve_static_function_binding_store_expression_with_context(
                    &raw_function_binding_expression,
                    call_snapshot_function_context.or_else(|| self.current_function_name()),
                )
            };
        let function_binding_expression = if self
            .expression_depends_on_active_loop_assignment(&resolved_function_binding_expression)
        {
            raw_function_binding_expression.clone()
        } else {
            resolved_function_binding_expression
        };
        let function_binding = if value_is_object_literal {
            None
        } else if self.expression_depends_on_active_loop_assignment(&function_binding_expression) {
            self.resolve_function_binding_from_expression_with_context(
                value_expression,
                call_snapshot_function_context,
            )
            .or(call_result_function_binding)
            .or_else(|| self.resolve_function_binding_from_expression(value_expression))
        } else {
            self.resolve_function_binding_from_expression_with_context(
                &function_binding_expression,
                call_snapshot_function_context,
            )
            .or(call_result_function_binding)
            .or_else(|| self.resolve_function_binding_from_expression(&function_binding_expression))
        };
        trace_timing("function_binding");
        if trace_identifier_store {
            eprintln!(
                "identifier_store:{name}:function_binding snapshot_context={call_snapshot_function_context:?} call_result={call_result_snapshot_expression:?} raw={raw_function_binding_expression:?} expr={function_binding_expression:?} binding={function_binding:?}"
            );
        }
        if matched_call_snapshot.is_some()
            && !local_simple_async_generator_next_call
            && resolved_descriptor_binding.is_none()
            && function_binding.is_some()
            && matches!(
                call_result_snapshot_expression.as_ref(),
                Some(Expression::Identifier(_) | Expression::This)
            )
        {
            let tracked_function_value = call_result_snapshot_expression
                .as_ref()
                .expect("matched Some above")
                .clone();
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:call_snapshot_function_fast_path");
            }
            return PreparedIdentifierValueStore {
                canonical_value_expression: canonical_value_expression.clone(),
                tracked_value_expression: tracked_function_value.clone(),
                descriptor_binding_expression: Expression::Undefined,
                tracked_object_expression: Expression::Undefined,
                call_source_snapshot_expression,
                prototype_source_snapshot_expression,
                function_binding_expression,
                function_binding,
                object_binding_expression: Expression::Undefined,
                object_binding: None,
                kind: Some(StaticValueKind::Function),
                static_string_value: None,
                exact_static_number: None,
                array_binding: None,
                module_assignment_expression: tracked_function_value,
                resolved_local_binding,
                returned_descriptor_binding: None,
                runtime_value_override: None,
                opaque_runtime_value: false,
            };
        }
        if matches!(canonical_value_expression, Expression::New { .. })
            && matched_call_snapshot.is_some_and(|snapshot| snapshot.result_expression.is_none())
        {
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:deferred_construct_snapshot");
            }
            let module_assignment_expression = canonical_value_expression.clone();
            return PreparedIdentifierValueStore {
                canonical_value_expression,
                tracked_value_expression,
                descriptor_binding_expression: Expression::Undefined,
                tracked_object_expression: Expression::Undefined,
                call_source_snapshot_expression,
                prototype_source_snapshot_expression,
                function_binding_expression,
                function_binding,
                object_binding_expression: Expression::Undefined,
                object_binding: None,
                kind: Some(StaticValueKind::Object),
                static_string_value: None,
                exact_static_number: None,
                array_binding: None,
                module_assignment_expression,
                resolved_local_binding,
                returned_descriptor_binding: None,
                runtime_value_override: None,
                opaque_runtime_value: false,
            };
        }
        let canonical_object_binding = if local_simple_async_generator_next_call {
            None
        } else {
            self.resolve_object_binding_from_expression(&canonical_value_expression)
        };
        trace_timing("canonical_object_binding");
        let returned_call_object_binding = if local_simple_async_generator_next_call {
            None
        } else if let Expression::Call { callee, arguments } = &canonical_value_expression {
            self.resolve_returned_object_binding_from_call(callee, arguments)
        } else {
            None
        };
        trace_timing("returned_call_object_binding");
        let resolved_construct_object_binding =
            if static_expression_matches(&function_binding_expression, &canonical_value_expression)
            {
                canonical_object_binding.clone()
            } else if matches!(
                function_binding_expression,
                Expression::Call { .. } | Expression::New { .. } | Expression::Object(_)
            ) {
                self.resolve_object_binding_from_expression(&function_binding_expression)
            } else {
                None
            };
        trace_timing("resolved_construct_object_binding");
        let mut object_binding_expression = if canonical_object_binding
            .as_ref()
            .is_some_and(|binding| self.object_binding_is_static_map(binding))
        {
            canonical_value_expression.clone()
        } else if call_result_snapshot_expression
            .as_ref()
            .is_some_and(Self::expression_contains_static_update)
            && let Some(canonical_object_binding) = canonical_object_binding.as_ref()
        {
            object_binding_to_expression(canonical_object_binding)
        } else {
            if call_result_has_function_binding
                && matches!(canonical_value_expression, Expression::New { .. })
                && canonical_object_binding.is_some()
            {
                canonical_value_expression.clone()
            } else {
                call_result_snapshot_expression
                    .as_ref()
                    .filter(|expression| {
                        self.resolve_object_binding_from_expression(expression)
                            .is_some()
                    })
                    .or_else(|| {
                        resolved_construct_object_binding
                            .as_ref()
                            .map(|_| &function_binding_expression)
                    })
                    .or_else(|| {
                        returned_call_object_binding
                            .as_ref()
                            .map(|_| &canonical_value_expression)
                    })
                    .unwrap_or(&tracked_object_expression)
                    .clone()
            }
        };
        if self.expression_depends_on_active_loop_assignment(&object_binding_expression) {
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:object_binding skipped_active_loop_dependency");
            }
            object_binding_expression = Expression::Undefined;
        }
        let object_binding =
            if static_expression_matches(&object_binding_expression, &canonical_value_expression) {
                canonical_object_binding
                    .clone()
                    .or_else(|| returned_call_object_binding.clone())
            } else if static_expression_matches(
                &object_binding_expression,
                &function_binding_expression,
            ) {
                resolved_construct_object_binding.clone()
            } else if matches!(object_binding_expression, Expression::Object(_)) {
                self.resolve_object_binding_from_expression(&object_binding_expression)
            } else {
                None
            }
            .map(|binding| self.normalize_prepared_object_binding_property_keys(binding));
        trace_timing("object_binding");
        if trace_identifier_store {
            eprintln!(
                "identifier_store:{name}:object_binding expr={object_binding_expression:?} prepared_binding={}",
                object_binding.is_some()
            );
            if let Some(binding) = object_binding.as_ref() {
                eprintln!(
                    "identifier_store:{name}:object_binding_contents {:?}",
                    object_binding_to_expression(binding)
                );
            }
        }
        let metadata_value_expression = call_result_snapshot_expression
            .as_ref()
            .unwrap_or(&tracked_value_expression);
        let mut kind = self.infer_value_kind(metadata_value_expression);
        if kind.is_none_or(|kind| kind == StaticValueKind::Unknown)
            && !self
                .runtime_string_print_candidates(metadata_value_expression)
                .is_empty()
        {
            kind = Some(StaticValueKind::String);
        }
        if trace_identifier_store {
            eprintln!("identifier_store:{name}:kind");
        }
        trace_timing("kind");
        let static_string_value = if kind == Some(StaticValueKind::String) {
            self.resolve_static_string_value(metadata_value_expression)
        } else {
            None
        };
        trace_timing("string");
        if trace_identifier_store {
            eprintln!("identifier_store:{name}:string");
        }
        let exact_static_number =
            (!Self::expression_contains_await_for_user_call_runtime(metadata_value_expression)
                && matches!(
                    kind,
                    Some(
                        StaticValueKind::Number
                            | StaticValueKind::BigInt
                            | StaticValueKind::Bool
                            | StaticValueKind::String
                            | StaticValueKind::Null
                            | StaticValueKind::Undefined
                    )
                ))
            .then(|| self.resolve_static_number_value(metadata_value_expression))
            .flatten()
            .filter(|number| {
                number.is_nan()
                    || !number.is_finite()
                    || number.fract() != 0.0
                    || (*number == 0.0 && number.is_sign_negative())
            });
        if trace_identifier_store {
            eprintln!("identifier_store:{name}:number");
        }
        trace_timing("number");
        let array_binding =
            if Self::expression_contains_await_for_user_call_runtime(metadata_value_expression) {
                None
            } else {
                self.resolve_array_binding_from_expression(metadata_value_expression)
            };
        trace_timing("array");
        if trace_identifier_store {
            eprintln!("identifier_store:{name}:array");
        }
        let preserve_tracked_member_expression = matches!(
            &tracked_value_expression,
            Expression::Member { object, property }
                if self.resolve_member_function_capture_slots(object, property).is_some()
                    || self
                        .object_literal_member_function_display_name(&tracked_value_expression, 0)
                        .is_some()
        );
        let module_assignment_expression = if matches!(
            &function_binding,
            Some(LocalFunctionBinding::Builtin(function_name))
                if parse_test262_realm_eval_builtin(function_name).is_some()
        ) {
            function_binding_expression.clone()
        } else if preserve_tracked_member_expression {
            tracked_value_expression.clone()
        } else if expression_is_dynamic_import_call(&tracked_value_expression) {
            tracked_value_expression.clone()
        } else if expression_is_dynamic_import_call(metadata_value_expression) {
            metadata_value_expression.clone()
        } else if matches!(
            call_result_snapshot_expression,
            Some(Expression::Identifier(_) | Expression::This)
        ) {
            call_result_snapshot_expression
                .as_ref()
                .expect("matched above")
                .clone()
        } else if Self::expression_contains_await_for_user_call_runtime(metadata_value_expression) {
            metadata_value_expression.clone()
        } else {
            self.materialize_static_expression(metadata_value_expression)
        };
        if trace_identifier_store {
            eprintln!("identifier_store:{name}:module");
        }
        trace_timing("module");
        PreparedIdentifierValueStore {
            canonical_value_expression,
            tracked_value_expression,
            descriptor_binding_expression,
            tracked_object_expression,
            call_source_snapshot_expression,
            prototype_source_snapshot_expression,
            function_binding_expression,
            function_binding,
            object_binding_expression,
            object_binding,
            kind,
            static_string_value,
            exact_static_number,
            array_binding,
            module_assignment_expression,
            resolved_local_binding,
            returned_descriptor_binding,
            runtime_value_override: None,
            opaque_runtime_value: false,
        }
    }
}

fn expression_is_statically_non_thenable(expression: &Expression) -> bool {
    match expression {
        Expression::Array(_)
        | Expression::String(_)
        | Expression::Number(_)
        | Expression::BigInt(_)
        | Expression::Bool(_)
        | Expression::Null
        | Expression::Undefined => true,
        Expression::Object(entries) => entries.iter().all(|entry| match entry {
            ObjectEntry::Data { key, .. }
            | ObjectEntry::Getter { key, .. }
            | ObjectEntry::Setter { key, .. } => {
                !matches!(key, Expression::String(name) if name == "then")
            }
            ObjectEntry::Spread(_) => false,
        }),
        _ => false,
    }
}
