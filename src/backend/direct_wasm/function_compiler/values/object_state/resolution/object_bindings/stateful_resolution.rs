use super::*;

thread_local! {
    static STATEFUL_OBJECT_BINDING_RESOLUTION_STACK: std::cell::RefCell<Vec<Expression>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

struct StatefulObjectBindingResolutionGuard;

impl StatefulObjectBindingResolutionGuard {
    fn enter(expression: &Expression) -> Option<Self> {
        let reentered = STATEFUL_OBJECT_BINDING_RESOLUTION_STACK.with(|stack| {
            stack
                .borrow()
                .iter()
                .any(|visited| static_expression_matches(visited, expression))
        });
        if reentered {
            return None;
        }
        STATEFUL_OBJECT_BINDING_RESOLUTION_STACK.with(|stack| {
            stack.borrow_mut().push(expression.clone());
        });
        Some(Self)
    }
}

impl Drop for StatefulObjectBindingResolutionGuard {
    fn drop(&mut self) {
        STATEFUL_OBJECT_BINDING_RESOLUTION_STACK.with(|stack| {
            stack.borrow_mut().pop();
        });
    }
}

fn call_argument_contains_static_match(argument: &CallArgument, target: &Expression) -> bool {
    match argument {
        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
            expression_contains_static_match(expression, target)
        }
    }
}

fn array_element_contains_static_match(element: &ArrayElement, target: &Expression) -> bool {
    match element {
        ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
            expression_contains_static_match(expression, target)
        }
    }
}

fn object_entry_contains_static_match(entry: &ObjectEntry, target: &Expression) -> bool {
    match entry {
        ObjectEntry::Data { key, value } => {
            expression_contains_static_match(key, target)
                || expression_contains_static_match(value, target)
        }
        ObjectEntry::Getter { key, getter } => {
            expression_contains_static_match(key, target)
                || expression_contains_static_match(getter, target)
        }
        ObjectEntry::Setter { key, setter } => {
            expression_contains_static_match(key, target)
                || expression_contains_static_match(setter, target)
        }
        ObjectEntry::Spread(expression) => expression_contains_static_match(expression, target),
    }
}

fn expression_contains_static_match(expression: &Expression, target: &Expression) -> bool {
    if static_expression_matches(expression, target) {
        return true;
    }
    match expression {
        Expression::Array(elements) => elements
            .iter()
            .any(|element| array_element_contains_static_match(element, target)),
        Expression::Object(entries) => entries
            .iter()
            .any(|entry| object_entry_contains_static_match(entry, target)),
        Expression::Member { object, property } => {
            expression_contains_static_match(object, target)
                || expression_contains_static_match(property, target)
        }
        Expression::SuperMember { property } => expression_contains_static_match(property, target),
        Expression::Assign { value, .. } | Expression::Await(value) => {
            expression_contains_static_match(value, target)
        }
        Expression::AssignMember {
            object,
            property,
            value,
        } => {
            expression_contains_static_match(object, target)
                || expression_contains_static_match(property, target)
                || expression_contains_static_match(value, target)
        }
        Expression::AssignSuperMember { property, value } => {
            expression_contains_static_match(property, target)
                || expression_contains_static_match(value, target)
        }
        Expression::EnumerateKeys(value)
        | Expression::GetIterator(value)
        | Expression::IteratorClose(value)
        | Expression::Unary {
            expression: value, ..
        } => expression_contains_static_match(value, target),
        Expression::Binary { left, right, .. } => {
            expression_contains_static_match(left, target)
                || expression_contains_static_match(right, target)
        }
        Expression::Conditional {
            condition,
            then_expression,
            else_expression,
        } => {
            expression_contains_static_match(condition, target)
                || expression_contains_static_match(then_expression, target)
                || expression_contains_static_match(else_expression, target)
        }
        Expression::Sequence(expressions) => expressions
            .iter()
            .any(|expression| expression_contains_static_match(expression, target)),
        Expression::Call { callee, arguments }
        | Expression::SuperCall { callee, arguments }
        | Expression::New { callee, arguments } => {
            expression_contains_static_match(callee, target)
                || arguments
                    .iter()
                    .any(|argument| call_argument_contains_static_match(argument, target))
        }
        Expression::Number(_)
        | Expression::BigInt(_)
        | Expression::String(_)
        | Expression::Bool(_)
        | Expression::Null
        | Expression::Undefined
        | Expression::NewTarget
        | Expression::Identifier(_)
        | Expression::This
        | Expression::Sent
        | Expression::Update { .. } => false,
    }
}

impl<'a> FunctionCompiler<'a> {
    fn resolve_array_binding_expression_with_state(
        &self,
        expression: &Expression,
        environment: &mut StaticResolutionEnvironment,
    ) -> Option<ArrayValueBinding> {
        let resolved = self
            .evaluate_static_expression_with_state(expression, environment)
            .or_else(|| self.materialize_static_expression_with_state(expression, environment))
            .unwrap_or_else(|| expression.clone());
        if let Expression::Identifier(name) = &resolved
            && let Some(value) = environment.binding(name)
            && !matches!(value, Expression::Identifier(alias) if alias == name)
            && let Some(binding) = self.resolve_array_binding_from_expression(value)
        {
            return Some(binding);
        }
        self.resolve_array_binding_from_expression(&resolved)
            .or_else(|| self.resolve_array_binding_from_expression(expression))
    }

    fn resolve_proxy_trap_return_expression_with_state(
        &self,
        binding: &LocalFunctionBinding,
        arguments: &[CallArgument],
        environment: &mut StaticResolutionEnvironment,
    ) -> Option<Expression> {
        let result = match binding {
            LocalFunctionBinding::User(function_name) => self
                .resolve_static_return_expression_from_user_function_call(
                    function_name,
                    arguments,
                    None,
                )
                .or_else(|| {
                    self.resolve_static_function_outcome_from_binding_with_context(
                        binding,
                        arguments,
                        self.current_function_name(),
                    )
                    .and_then(|outcome| match outcome {
                        StaticEvalOutcome::Value(value) => Some(value),
                        StaticEvalOutcome::Throw(_) => None,
                    })
                })?,
            LocalFunctionBinding::Builtin(_) => {
                match self.resolve_static_function_outcome_from_binding_with_context(
                    binding,
                    arguments,
                    self.current_function_name(),
                )? {
                    StaticEvalOutcome::Value(value) => value,
                    StaticEvalOutcome::Throw(_) => return None,
                }
            }
        };
        Some(
            self.evaluate_static_expression_with_state(&result, environment)
                .or_else(|| self.materialize_static_expression_with_state(&result, environment))
                .unwrap_or(result),
        )
    }

    fn resolve_proxy_binding_from_expression_with_state(
        &self,
        expression: &Expression,
        environment: &mut StaticResolutionEnvironment,
        depth: usize,
    ) -> Option<ProxyValueBinding> {
        if depth > 8 {
            return None;
        }
        if let Some(binding) = self.resolve_proxy_binding_from_expression(expression) {
            return Some(binding);
        }
        if let Expression::Identifier(name) = expression
            && let Some(value) = environment.binding(name).cloned()
            && !static_expression_matches(&value, expression)
            && let Some(binding) = self.resolve_proxy_binding_from_expression_with_state(
                &value,
                environment,
                depth + 1,
            )
        {
            return Some(binding);
        }
        let materialized = self
            .evaluate_static_expression_with_state(expression, environment)
            .or_else(|| self.materialize_static_expression_with_state(expression, environment));
        if let Some(materialized) = materialized
            && !static_expression_matches(&materialized, expression)
        {
            return self.resolve_proxy_binding_from_expression_with_state(
                &materialized,
                environment,
                depth + 1,
            );
        }
        None
    }

    fn materialize_proxy_copy_property_key_with_state(
        &self,
        key: &Expression,
        environment: &mut StaticResolutionEnvironment,
    ) -> Option<Expression> {
        let materialized = self
            .evaluate_static_expression_with_state(key, environment)
            .or_else(|| self.materialize_static_expression_with_state(key, environment))
            .unwrap_or_else(|| key.clone());
        self.resolve_property_key_expression(&materialized)
            .or_else(|| {
                self.static_eval_context()
                    .resolve_property_key_with_state(&materialized, environment)
            })
            .or_else(|| static_property_name_from_expression(&materialized).map(Expression::String))
            .or(Some(materialized))
    }

    fn evaluate_proxy_descriptor_field_with_state(
        &self,
        expression: &Expression,
        environment: &mut StaticResolutionEnvironment,
    ) -> Expression {
        if let Some(value) =
            self.evaluate_proxy_descriptor_array_index_of_call_with_state(expression, environment)
        {
            return value;
        }

        match expression {
            Expression::Unary { op, expression } => {
                let value =
                    self.evaluate_proxy_descriptor_field_with_state(expression, environment);
                match (op, value) {
                    (UnaryOp::Not, Expression::Bool(value)) => Expression::Bool(!value),
                    (UnaryOp::Plus, Expression::Number(value)) => Expression::Number(value),
                    (UnaryOp::Negate, Expression::Number(value)) => Expression::Number(-value),
                    (UnaryOp::Plus | UnaryOp::Negate, value) => self
                        .resolve_static_number_value(&value)
                        .map(|number| {
                            Expression::Number(if matches!(op, UnaryOp::Negate) {
                                -number
                            } else {
                                number
                            })
                        })
                        .unwrap_or_else(|| Expression::Unary {
                            op: *op,
                            expression: Box::new(value),
                        }),
                    (_, value) => Expression::Unary {
                        op: *op,
                        expression: Box::new(value),
                    },
                }
            }
            Expression::Binary { op, left, right } => {
                let left = self.evaluate_proxy_descriptor_field_with_state(left, environment);
                let right = self.evaluate_proxy_descriptor_field_with_state(right, environment);
                let expression = Expression::Binary {
                    op: *op,
                    left: Box::new(left),
                    right: Box::new(right),
                };
                self.evaluate_static_expression_with_state(&expression, environment)
                    .or_else(|| {
                        self.resolve_static_boolean_expression(&expression)
                            .map(Expression::Bool)
                    })
                    .or_else(|| {
                        self.resolve_static_number_value(&expression)
                            .map(Expression::Number)
                    })
                    .unwrap_or(expression)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                let condition =
                    self.evaluate_proxy_descriptor_field_with_state(condition, environment);
                if let Some(condition) = self.resolve_static_boolean_expression(&condition) {
                    self.evaluate_proxy_descriptor_field_with_state(
                        if condition {
                            then_expression
                        } else {
                            else_expression
                        },
                        environment,
                    )
                } else {
                    Expression::Conditional {
                        condition: Box::new(condition),
                        then_expression: Box::new(self.evaluate_proxy_descriptor_field_with_state(
                            then_expression,
                            environment,
                        )),
                        else_expression: Box::new(self.evaluate_proxy_descriptor_field_with_state(
                            else_expression,
                            environment,
                        )),
                    }
                }
            }
            Expression::Object(entries) => {
                let entries = entries
                    .iter()
                    .map(|entry| match entry {
                        ObjectEntry::Data { key, value } => ObjectEntry::Data {
                            key: self.evaluate_proxy_descriptor_field_with_state(key, environment),
                            value: self
                                .evaluate_proxy_descriptor_field_with_state(value, environment),
                        },
                        ObjectEntry::Getter { key, getter } => ObjectEntry::Getter {
                            key: self.evaluate_proxy_descriptor_field_with_state(key, environment),
                            getter: getter.clone(),
                        },
                        ObjectEntry::Setter { key, setter } => ObjectEntry::Setter {
                            key: self.evaluate_proxy_descriptor_field_with_state(key, environment),
                            setter: setter.clone(),
                        },
                        ObjectEntry::Spread(expression) => {
                            ObjectEntry::Spread(self.evaluate_proxy_descriptor_field_with_state(
                                expression,
                                environment,
                            ))
                        }
                    })
                    .collect();
                Expression::Object(entries)
            }
            Expression::Array(elements) => {
                let elements = elements
                    .iter()
                    .map(|element| match element {
                        ArrayElement::Expression(expression) => ArrayElement::Expression(
                            self.evaluate_proxy_descriptor_field_with_state(
                                expression,
                                environment,
                            ),
                        ),
                        ArrayElement::Spread(expression) => {
                            ArrayElement::Spread(self.evaluate_proxy_descriptor_field_with_state(
                                expression,
                                environment,
                            ))
                        }
                    })
                    .collect();
                Expression::Array(elements)
            }
            _ => self
                .evaluate_static_expression_with_state(expression, environment)
                .or_else(|| self.materialize_static_expression_with_state(expression, environment))
                .unwrap_or_else(|| self.materialize_static_expression(expression)),
        }
    }

    fn evaluate_proxy_descriptor_array_index_of_call_with_state(
        &self,
        expression: &Expression,
        environment: &mut StaticResolutionEnvironment,
    ) -> Option<Expression> {
        let Expression::Call { callee, arguments } = expression else {
            return None;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return None;
        };
        if !matches!(property.as_ref(), Expression::String(name) if name == "indexOf") {
            return None;
        }

        let search_expression = match arguments.first() {
            Some(CallArgument::Expression(expression) | CallArgument::Spread(expression)) => {
                expression
            }
            None => return Some(Expression::Number(-1.0)),
        };
        let search_value =
            self.evaluate_proxy_descriptor_field_with_state(search_expression, environment);
        let search_value = self
            .static_eval_context()
            .resolve_property_key_with_state(&search_value, environment)
            .unwrap_or(search_value);
        let array_binding =
            self.resolve_array_binding_expression_with_state(object, environment)?;
        let found_index = array_binding
            .values
            .iter()
            .enumerate()
            .find_map(|(index, value)| {
                let value = value.as_ref()?;
                let value = self.evaluate_proxy_descriptor_field_with_state(value, environment);
                let value = self
                    .static_eval_context()
                    .resolve_property_key_with_state(&value, environment)
                    .unwrap_or(value);
                static_expression_matches(&value, &search_value).then_some(index as f64)
            })
            .unwrap_or(-1.0);
        Some(Expression::Number(found_index))
    }

    fn proxy_descriptor_boolean_field_with_state(
        &self,
        expression: &Expression,
        environment: &mut StaticResolutionEnvironment,
    ) -> Option<bool> {
        let value = self.evaluate_proxy_descriptor_field_with_state(expression, environment);
        match value {
            Expression::Bool(value) => Some(value),
            _ => self.resolve_static_boolean_expression(&value),
        }
    }

    fn resolve_proxy_property_descriptor_definition_with_state(
        &self,
        expression: &Expression,
        environment: &mut StaticResolutionEnvironment,
    ) -> Option<PropertyDescriptorDefinition> {
        let descriptor_expression =
            self.evaluate_proxy_descriptor_field_with_state(expression, environment);
        let Expression::Object(entries) = descriptor_expression else {
            return None;
        };

        let mut descriptor = PropertyDescriptorDefinition::default();
        for entry in entries {
            match entry {
                ObjectEntry::Data { key, value } => {
                    let key = self.evaluate_proxy_descriptor_field_with_state(&key, environment);
                    let Some(key_name) = static_property_name_from_expression(&key) else {
                        return None;
                    };
                    match key_name.as_str() {
                        "value" => {
                            descriptor.value =
                                Some(self.evaluate_proxy_descriptor_field_with_state(
                                    &value,
                                    environment,
                                ));
                        }
                        "writable" => {
                            descriptor.writable =
                                Some(self.proxy_descriptor_boolean_field_with_state(
                                    &value,
                                    environment,
                                )?);
                        }
                        "enumerable" => {
                            descriptor.enumerable =
                                Some(self.proxy_descriptor_boolean_field_with_state(
                                    &value,
                                    environment,
                                )?);
                        }
                        "configurable" => {
                            descriptor.configurable =
                                Some(self.proxy_descriptor_boolean_field_with_state(
                                    &value,
                                    environment,
                                )?);
                        }
                        "get" => {
                            descriptor.getter =
                                Some(self.evaluate_proxy_descriptor_field_with_state(
                                    &value,
                                    environment,
                                ));
                        }
                        "set" => {
                            descriptor.setter =
                                Some(self.evaluate_proxy_descriptor_field_with_state(
                                    &value,
                                    environment,
                                ));
                        }
                        key if key.starts_with("__ayy$") => {}
                        _ => return None,
                    }
                }
                ObjectEntry::Getter { key, getter } => {
                    let key = self.evaluate_proxy_descriptor_field_with_state(&key, environment);
                    if !matches!(key, Expression::String(ref key_name) if key_name == "get") {
                        return None;
                    }
                    descriptor.getter = Some(getter);
                }
                ObjectEntry::Setter { key, setter } => {
                    let key = self.evaluate_proxy_descriptor_field_with_state(&key, environment);
                    if !matches!(key, Expression::String(ref key_name) if key_name == "set") {
                        return None;
                    }
                    descriptor.setter = Some(setter);
                }
                ObjectEntry::Spread(_) => return None,
            }
        }

        Some(descriptor)
    }

    fn resolve_simple_user_function_return_alias(
        &self,
        function_name: &str,
        arguments: &[CallArgument],
    ) -> Option<Expression> {
        let user_function = self.user_function(function_name)?;
        let function = self.resolve_registered_function_declaration(function_name)?;
        let return_expression = function.body.iter().rev().find_map(|statement| {
            if let Statement::Return(expression) = statement {
                Some(expression)
            } else {
                None
            }
        })?;
        Some(self.substitute_user_function_call_frame_bindings(
            return_expression,
            user_function,
            arguments,
            &Expression::Undefined,
            &Expression::Array(Vec::new()),
        ))
    }

    fn property_descriptor_binding_from_proxy_result_with_state(
        &self,
        descriptor_expression: &Expression,
        environment: &mut StaticResolutionEnvironment,
    ) -> Option<Option<PropertyDescriptorBinding>> {
        let descriptor_expression = self
            .evaluate_static_expression_with_state(descriptor_expression, environment)
            .or_else(|| {
                self.materialize_static_expression_with_state(descriptor_expression, environment)
            })
            .unwrap_or_else(|| descriptor_expression.clone());
        if matches!(
            descriptor_expression,
            Expression::Undefined | Expression::Null
        ) {
            return Some(None);
        }
        let descriptor = self.resolve_proxy_property_descriptor_definition_with_state(
            &descriptor_expression,
            environment,
        )?;
        let descriptor_value =
            |expression: &Expression,
             compiler: &Self,
             environment: &mut StaticResolutionEnvironment| {
                compiler
                    .evaluate_static_expression_with_state(expression, environment)
                    .or_else(|| {
                        compiler.materialize_static_expression_with_state(expression, environment)
                    })
                    .unwrap_or_else(|| expression.clone())
            };
        let value = descriptor
            .value
            .as_ref()
            .map(|expression| descriptor_value(expression, self, environment));
        let getter = descriptor
            .getter
            .as_ref()
            .map(|expression| descriptor_value(expression, self, environment));
        let setter = descriptor
            .setter
            .as_ref()
            .map(|expression| descriptor_value(expression, self, environment));
        Some(Some(PropertyDescriptorBinding {
            value,
            configurable: descriptor.configurable.unwrap_or(false),
            enumerable: descriptor.enumerable.unwrap_or(false),
            writable: if descriptor.is_accessor() {
                None
            } else {
                Some(descriptor.writable.unwrap_or(false))
            },
            getter,
            setter,
            has_get: descriptor.getter.is_some(),
            has_set: descriptor.setter.is_some(),
        }))
    }

    fn resolve_proxy_copy_data_properties_binding_with_state(
        &self,
        expression: &Expression,
        environment: &mut StaticResolutionEnvironment,
    ) -> Option<ObjectValueBinding> {
        static TRACE_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let trace_copy_data = std::env::var_os("AYY_TRACE_COPY_DATA").is_some()
            && TRACE_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed) < 4;
        let proxy_binding =
            self.resolve_proxy_binding_from_expression_with_state(expression, environment, 0)?;
        if trace_copy_data {
            eprintln!(
                "copy_data:proxy_start expression={expression:?} target={:?} handler={:?} get={} gopd={} own_keys={}",
                proxy_binding.target,
                proxy_binding.handler,
                proxy_binding.get_binding.is_some(),
                proxy_binding.get_own_property_descriptor_binding.is_some(),
                proxy_binding.own_keys_binding.is_some()
            );
        }
        let Some(own_keys_binding) = proxy_binding.own_keys_binding.as_ref() else {
            if trace_copy_data {
                eprintln!("copy_data:proxy_abort missing_own_keys");
            }
            return None;
        };
        let own_keys_arguments = [CallArgument::Expression(proxy_binding.target.clone())];
        let Some(own_keys_expression) = self.resolve_proxy_trap_return_expression_with_state(
            own_keys_binding,
            &own_keys_arguments,
            environment,
        ) else {
            if trace_copy_data {
                eprintln!("copy_data:proxy_abort own_keys_return");
            }
            return None;
        };
        if trace_copy_data {
            eprintln!("copy_data:proxy_own_keys_expression={own_keys_expression:?}");
        }
        let Some(own_keys) =
            self.resolve_array_binding_expression_with_state(&own_keys_expression, environment)
        else {
            if trace_copy_data {
                eprintln!("copy_data:proxy_abort own_keys_array");
            }
            return None;
        };
        if trace_copy_data {
            eprintln!("copy_data:proxy_own_keys_values={:?}", own_keys.values);
        }
        let mut copied_binding = empty_object_value_binding();

        for key in own_keys.values.into_iter().flatten() {
            let Some(property) =
                self.materialize_proxy_copy_property_key_with_state(&key, environment)
            else {
                if trace_copy_data {
                    eprintln!("copy_data:proxy_abort property_key key={key:?}");
                }
                return None;
            };
            if trace_copy_data {
                eprintln!("copy_data:proxy_key key={key:?} property={property:?}");
            }
            let descriptor = if let Some(descriptor_binding) =
                proxy_binding.get_own_property_descriptor_binding.as_ref()
            {
                let descriptor_arguments = [
                    CallArgument::Expression(proxy_binding.target.clone()),
                    CallArgument::Expression(property.clone()),
                ];
                let Some(descriptor_expression) = self
                    .resolve_proxy_trap_return_expression_with_state(
                        descriptor_binding,
                        &descriptor_arguments,
                        environment,
                    )
                else {
                    if trace_copy_data {
                        eprintln!("copy_data:proxy_abort descriptor_return property={property:?}");
                    }
                    return None;
                };
                if trace_copy_data {
                    eprintln!(
                        "copy_data:proxy_descriptor_expression property={property:?} expr={descriptor_expression:?}"
                    );
                }
                let descriptor = self.property_descriptor_binding_from_proxy_result_with_state(
                    &descriptor_expression,
                    environment,
                )?;
                if trace_copy_data {
                    eprintln!(
                        "copy_data:proxy_descriptor property={property:?} present={} enumerable={:?}",
                        descriptor.is_some(),
                        descriptor.as_ref().map(|descriptor| descriptor.enumerable)
                    );
                }
                descriptor
            } else {
                let target_binding = self.resolve_object_binding_from_expression_with_state(
                    &proxy_binding.target,
                    environment,
                )?;
                object_binding_lookup_descriptor(&target_binding, &property).cloned()
            };
            let Some(descriptor) = descriptor else {
                continue;
            };
            if !descriptor.enumerable {
                continue;
            }
            let value = if let Some(get_binding) = proxy_binding.get_binding.as_ref() {
                let get_arguments = [
                    CallArgument::Expression(proxy_binding.target.clone()),
                    CallArgument::Expression(property.clone()),
                    CallArgument::Expression(expression.clone()),
                ];
                match get_binding {
                    LocalFunctionBinding::User(function_name) => self
                        .resolve_simple_user_function_return_alias(function_name, &get_arguments)
                        .or_else(|| {
                            self.resolve_static_return_expression_from_user_function_call(
                                function_name,
                                &get_arguments,
                                None,
                            )
                        })
                        .or_else(|| {
                            self.resolve_proxy_trap_return_expression_with_state(
                                get_binding,
                                &get_arguments,
                                environment,
                            )
                        }),
                    LocalFunctionBinding::Builtin(_) => self
                        .resolve_proxy_trap_return_expression_with_state(
                            get_binding,
                            &get_arguments,
                            environment,
                        ),
                }
                .or_else(|| {
                    trace_copy_data.then(|| {
                        eprintln!("copy_data:proxy_abort get_return property={property:?}")
                    });
                    None
                })?
            } else {
                descriptor
                    .value
                    .clone()
                    .or_else(|| {
                        object_binding_lookup_value(
                            &self.resolve_object_binding_from_expression_with_state(
                                &proxy_binding.target,
                                environment,
                            )?,
                            &property,
                        )
                        .cloned()
                    })
                    .unwrap_or(Expression::Undefined)
            };
            if trace_copy_data {
                eprintln!("copy_data:proxy property={property:?} enumerable=true value={value:?}");
            }
            object_binding_define_copied_data_property(&mut copied_binding, property, value);
        }

        Some(copied_binding)
    }

    fn rematerialize_call_like_expression_with_state(
        &self,
        expression: &Expression,
        environment: &StaticResolutionEnvironment,
    ) -> Option<Expression> {
        match expression {
            Expression::Call { callee, arguments } => Some(Expression::Call {
                callee: Box::new(
                    self.materialize_static_expression_with_state(callee, environment)?,
                ),
                arguments: arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(expression) => self
                            .materialize_static_expression_with_state(expression, environment)
                            .map(CallArgument::Expression),
                        CallArgument::Spread(expression) => self
                            .materialize_static_expression_with_state(expression, environment)
                            .map(CallArgument::Spread),
                    })
                    .collect::<Option<Vec<_>>>()?,
            }),
            Expression::New { callee, arguments } => Some(Expression::New {
                callee: Box::new(
                    self.materialize_static_expression_with_state(callee, environment)?,
                ),
                arguments: arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(expression) => self
                            .materialize_static_expression_with_state(expression, environment)
                            .map(CallArgument::Expression),
                        CallArgument::Spread(expression) => self
                            .materialize_static_expression_with_state(expression, environment)
                            .map(CallArgument::Spread),
                    })
                    .collect::<Option<Vec<_>>>()?,
            }),
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_object_binding_entries_with_state(
        &self,
        entries: &[ObjectEntry],
        environment: &mut StaticResolutionEnvironment,
    ) -> Option<ObjectValueBinding> {
        resolve_structural_object_binding(
            entries,
            environment,
            |expression, environment| {
                if matches!(
                    expression,
                    Expression::Identifier(name)
                        if self
                            .runtime_object_property_shadow_owner_name_for_identifier(name)
                            .is_some()
                ) {
                    return Some(expression.clone());
                }
                if self.resolve_iterator_source_kind(expression).is_some() {
                    return Some(expression.clone());
                }
                self.evaluate_static_expression_with_state(expression, environment)
                    .or_else(|| {
                        self.materialize_static_expression_with_state(expression, environment)
                    })
            },
            |expression, _environment| {
                let mut environment = _environment.fork();
                let preserves_proxy = self
                    .resolve_proxy_binding_from_expression_with_state(
                        expression,
                        &mut environment,
                        0,
                    )
                    .is_some();
                if preserves_proxy {
                    return true;
                }
                self.iterator_step_member_static_value_binding_candidates(expression)
                    .iter()
                    .any(|candidate| {
                        matches!(candidate, Expression::Object(_) | Expression::Array(_))
                            || self
                                .resolve_object_binding_from_expression(candidate)
                                .is_some()
                    })
            },
            |spread_expression, _environment| {
                matches!(
                    spread_expression,
                    Expression::Identifier(name)
                        if name == "undefined"
                            && self.is_unshadowed_builtin_identifier(name)
                )
            },
            |spread_expression, environment| {
                let resolve_copy_data_properties =
                    |source: &Expression, environment: &mut StaticResolutionEnvironment| {
                        let proxy = self
                            .resolve_proxy_binding_from_expression_with_state(
                                source,
                                environment,
                                0,
                            )
                            .is_some();
                        if proxy {
                            return self
                                .resolve_proxy_copy_data_properties_binding_with_state(
                                    source,
                                    environment,
                                );
                        }
                        resolve_copy_data_properties_binding(
                            source,
                            environment,
                            |expression, environment| {
                                self.resolve_object_binding_from_expression_with_state(
                                    expression,
                                    environment,
                                )
                            },
                            |object, property, environment| {
                                let trace_copy_data =
                                    std::env::var_os("AYY_TRACE_COPY_DATA").is_some();
                                let binding =
                                    self.resolve_member_getter_binding(object, property)?;
                                if trace_copy_data {
                                    eprintln!(
                                        "copy_data:getter object={object:?} property={property:?} binding={binding:?}"
                                    );
                                }
                                let context = self.static_eval_context();
                                let executed = execute_static_user_function_binding_in_environment(
                                    &context,
                                    &binding,
                                    &[],
                                    environment,
                                    StaticFunctionEffectMode::Commit,
                                );
                                if trace_copy_data {
                                    eprintln!("copy_data:executed result={executed:?}");
                                }
                                let resolved = executed.or_else(|| match &binding {
                                    LocalFunctionBinding::User(function_name) => {
                                        let raw_return = self
                                            .resolve_static_return_expression_from_user_function_call(
                                                function_name,
                                                &[],
                                                None,
                                            )?;
                                        self.evaluate_static_expression_with_state(
                                            &raw_return,
                                            environment,
                                        )
                                        .or_else(|| {
                                            self.materialize_static_expression_with_state(
                                                &raw_return,
                                                environment,
                                            )
                                            .filter(|materialized| {
                                                inline_summary_side_effect_free_expression(
                                                    materialized,
                                                )
                                            })
                                        })
                                        .or_else(|| {
                                            inline_summary_side_effect_free_expression(&raw_return)
                                                .then_some(raw_return)
                                        })
                                    }
                                    LocalFunctionBinding::Builtin(_) => None,
                                });
                                if trace_copy_data {
                                    eprintln!("copy_data:resolved result={resolved:?}");
                                }
                                resolved
                            },
                        )
                    };
                for candidate in
                    self.iterator_step_member_static_value_binding_candidates(spread_expression)
                {
                    let materialized_candidate = self
                        .materialize_static_expression_with_state(&candidate, environment)
                        .unwrap_or_else(|| self.materialize_static_expression(&candidate));
                    let binding =
                        resolve_copy_data_properties(&materialized_candidate, environment);
                    if let Some(binding) = binding {
                        return Some(binding);
                    }
                }
                let materialized_spread_expression = self
                    .materialize_static_expression_with_state(spread_expression, environment)
                    .unwrap_or_else(|| self.materialize_static_expression(spread_expression));
                resolve_copy_data_properties(&materialized_spread_expression, environment)
                    .or_else(|| resolve_copy_data_properties(spread_expression, environment))
            },
        )
        .map(|binding| self.canonicalize_contextual_object_binding_property_keys(binding))
    }

    fn resolve_raw_member_value_with_state(
        &self,
        object: &Expression,
        property: &Expression,
        environment: &StaticResolutionEnvironment,
    ) -> Option<Expression> {
        let Expression::Identifier(object_name) = object else {
            return None;
        };
        let property = self
            .materialize_static_expression_with_state(property, environment)
            .unwrap_or_else(|| self.materialize_static_expression(property));
        if let Some(object_binding) = environment.object_binding(object_name)
            && let Some(value) = object_binding_lookup_value(object_binding, &property)
        {
            return Some(value.clone());
        }
        let object_value = environment
            .local_binding(object_name)
            .or_else(|| environment.global_value_binding(object_name))?;
        let Expression::Object(entries) = object_value else {
            return None;
        };
        for entry in entries {
            let ObjectEntry::Data { key, value } = entry else {
                continue;
            };
            let key = self
                .materialize_static_expression_with_state(key, environment)
                .unwrap_or_else(|| self.materialize_static_expression(key));
            if static_expression_matches(&key, &property) {
                return Some(value.clone());
            }
        }
        None
    }

    pub(in crate::backend::direct_wasm) fn resolve_object_binding_from_expression_with_state(
        &self,
        expression: &Expression,
        environment: &mut StaticResolutionEnvironment,
    ) -> Option<ObjectValueBinding> {
        let _guard = StatefulObjectBindingResolutionGuard::enter(expression)?;

        if let Expression::Identifier(name) = expression
            && self.current_function_name().is_some()
            && self.resolve_current_local_binding(name).is_none()
            && self
                .runtime_object_property_shadow_owner_name_for_identifier(name)
                .is_some()
            && let Some(binding) = self.resolve_object_binding_from_expression(expression)
        {
            return Some(binding);
        }

        if let Expression::Member { object, property } = expression {
            if let Some(value) =
                self.resolve_raw_member_value_with_state(object, property, environment)
                && !static_expression_matches(&value, expression)
            {
                if let Expression::Identifier(name) = &value
                    && let Some(binding) = self.resolve_runtime_shadow_object_binding(name)
                {
                    return Some(binding);
                }
                if let Some(binding) =
                    self.resolve_object_binding_from_expression_with_state(&value, environment)
                {
                    return Some(binding);
                }
            }
        }

        if let Expression::Await(value) = expression {
            if let Some(binding) =
                self.resolve_object_binding_from_expression_with_state(value, environment)
            {
                return Some(binding);
            }
            let materialized = self
                .materialize_static_expression_with_state(value, environment)
                .unwrap_or_else(|| self.materialize_static_expression(value));
            if let Some(binding) =
                self.resolve_object_binding_from_expression_with_state(&materialized, environment)
            {
                return Some(binding);
            }
            if let Some(StaticEvalOutcome::Value(value)) = self
                .resolve_static_await_resolution_outcome(&Expression::Await(Box::new(materialized)))
            {
                return self.resolve_object_binding_from_expression_with_state(&value, environment);
            }
        }

        if let Some(rematerialized) =
            self.rematerialize_call_like_expression_with_state(expression, environment)
            && !static_expression_matches(&rematerialized, expression)
        {
            if expression_contains_static_match(&rematerialized, expression) {
                return None;
            }
            return self
                .resolve_object_binding_from_expression_with_state(&rematerialized, environment);
        }

        if let Some(descriptor) =
            self.resolve_descriptor_binding_from_expression_with_state(expression, environment)
        {
            return Some(self.object_binding_from_property_descriptor(&descriptor));
        }

        resolve_stateful_object_binding_from_environment(
            expression,
            environment,
            &|expression, environment| {
                resolve_specialized_object_binding_expression(
                    expression,
                    environment,
                    |expression, _| self.resolve_array_binding_from_expression(expression),
                    |entries, environment| {
                        let mut environment = environment.fork();
                        self.resolve_object_binding_entries_with_state(entries, &mut environment)
                    },
                    |expression, environment| {
                        matches!(
                            expression,
                            Expression::Call { callee, .. }
                                if matches!(
                                    self.resolve_bound_alias_expression_with_state(
                                        callee,
                                        environment,
                                    )
                                    .as_ref(),
                                    Some(Expression::Member { object, property })
                                        if matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
                                            && matches!(property.as_ref(), Expression::String(name) if name == "create")
                                )
                        )
                    },
                    |expression, _| self.resolve_object_binding_from_expression(expression),
                )
            },
        )
        .or_else(|| self.resolve_object_binding_from_expression(expression))
    }
}
