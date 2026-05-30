use super::*;

impl DirectWasmCompiler {
    pub(in crate::backend::direct_wasm) fn register_parameter_value_bindings_for_call(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
        aliases: &HashMap<String, Option<LocalFunctionBinding>>,
        bindings: &mut HashMap<String, HashMap<String, Option<Expression>>>,
        source_bindings: &HashMap<String, HashMap<String, Option<Expression>>>,
        current_function_name: Option<&str>,
    ) {
        let dynamic_import_callback =
            self.dynamic_import_then_callback_namespace_argument(callee, arguments, aliases);
        let (called_function_name, call_arguments) = if let Some((
            called_function_name,
            namespace_argument,
        )) = dynamic_import_callback
        {
            (called_function_name, vec![namespace_argument])
        } else {
            match callee {
                Expression::Member { object, property } if matches!(property.as_ref(), Expression::String(name) if name == "call") =>
                {
                    let Some(LocalFunctionBinding::User(called_function_name)) =
                        self.resolve_function_binding_from_expression_with_aliases(object, aliases)
                    else {
                        return;
                    };
                    (
                        called_function_name,
                        self.expanded_global_static_call_arguments(arguments)
                            .into_iter()
                            .skip(1)
                            .collect::<Vec<_>>(),
                    )
                }
                Expression::Member { object, property } if matches!(property.as_ref(), Expression::String(name) if name == "apply") =>
                {
                    let Some(LocalFunctionBinding::User(called_function_name)) =
                        self.resolve_function_binding_from_expression_with_aliases(object, aliases)
                    else {
                        return;
                    };
                    let expanded_arguments = self.expanded_global_static_call_arguments(arguments);
                    let apply_expression = expanded_arguments
                        .get(1)
                        .cloned()
                        .unwrap_or(Expression::Undefined);
                    let Some(call_arguments) = self
                        .expand_apply_parameter_call_arguments_from_expression(&apply_expression)
                    else {
                        return;
                    };
                    (called_function_name, call_arguments)
                }
                _ => {
                    let Some(LocalFunctionBinding::User(called_function_name)) =
                        self.resolve_function_binding_from_expression_with_aliases(callee, aliases)
                    else {
                        return;
                    };
                    (
                        called_function_name,
                        self.expanded_global_static_call_arguments(arguments),
                    )
                }
            }
        };
        self.register_parameter_value_binding_candidates(
            &called_function_name,
            &call_arguments,
            bindings,
            source_bindings,
            current_function_name,
        );
    }

    pub(in crate::backend::direct_wasm) fn register_constructor_parameter_value_bindings_for_call(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
        aliases: &HashMap<String, Option<LocalFunctionBinding>>,
        bindings: &mut HashMap<String, HashMap<String, Option<Expression>>>,
        source_bindings: &HashMap<String, HashMap<String, Option<Expression>>>,
        current_function_name: Option<&str>,
    ) {
        let constructor_binding =
            self.resolve_parameter_analysis_constructor_binding(callee, aliases);
        let Some(LocalFunctionBinding::User(called_function_name)) = constructor_binding else {
            return;
        };
        let call_arguments = self.expanded_global_static_call_arguments(arguments);
        self.register_parameter_value_binding_candidates(
            &called_function_name,
            &call_arguments,
            bindings,
            source_bindings,
            current_function_name,
        );
    }

    pub(in crate::backend::direct_wasm) fn resolve_parameter_analysis_constructor_binding(
        &self,
        callee: &Expression,
        aliases: &HashMap<String, Option<LocalFunctionBinding>>,
    ) -> Option<LocalFunctionBinding> {
        self.resolve_function_binding_from_expression_with_aliases(callee, aliases)
            .or_else(|| {
                let Expression::Identifier(name) = callee else {
                    return None;
                };
                let resolved = self.resolve_static_class_init_local_alias_expression(name)?;
                self.resolve_function_binding_from_expression_with_aliases(&resolved, aliases)
            })
            .or_else(|| {
                let Expression::Identifier(name) = callee else {
                    return None;
                };
                self.resolve_class_constructor_self_binding_for_parameter_analysis(name)
            })
    }

    fn resolve_class_constructor_self_binding_for_parameter_analysis(
        &self,
        name: &str,
    ) -> Option<LocalFunctionBinding> {
        let source_name = scoped_binding_source_name(name);
        self.state.user_functions().iter().find_map(|function| {
            if !function.name.starts_with("__ayy_class_ctor_") {
                return None;
            }
            let declaration = self.registered_function(&function.name)?;
            declaration
                .self_binding
                .as_deref()
                .filter(|self_binding| {
                    *self_binding == name
                        || source_name.is_some_and(|source_name| *self_binding == source_name)
                        || scoped_binding_source_name(self_binding) == Some(name)
                })
                .map(|_| LocalFunctionBinding::User(function.name.clone()))
        })
    }

    fn register_parameter_value_binding_candidates(
        &self,
        called_function_name: &str,
        call_arguments: &[Expression],
        bindings: &mut HashMap<String, HashMap<String, Option<Expression>>>,
        source_bindings: &HashMap<String, HashMap<String, Option<Expression>>>,
        current_function_name: Option<&str>,
    ) {
        let Some(user_function) = self.user_function(called_function_name) else {
            return;
        };
        let current_function_value_bindings = current_function_name
            .and_then(|name| source_bindings.get(name))
            .cloned()
            .unwrap_or_default();
        let Some(parameter_bindings) = bindings.get_mut(called_function_name) else {
            return;
        };
        let rest_parameter_index =
            self.registered_function(called_function_name)
                .and_then(|declaration| {
                    declaration
                        .params
                        .iter()
                        .position(|parameter| parameter.rest)
                });

        let mut handled_rest_parameter = false;
        for (index, argument) in call_arguments.iter().enumerate() {
            if index >= user_function.params.len() {
                break;
            }
            let param_name = &user_function.params[index];
            if rest_parameter_index == Some(index) {
                let rest_values = call_arguments[index..]
                    .iter()
                    .map(|argument| {
                        self.effective_parameter_value_binding_argument(
                            argument,
                            &current_function_value_bindings,
                        )
                    })
                    .collect::<Option<Vec<_>>>();
                let candidate = rest_values.map(|values| {
                    Expression::Array(
                        values
                            .into_iter()
                            .map(ArrayElement::Expression)
                            .collect::<Vec<_>>(),
                    )
                });
                Self::merge_parameter_value_binding_candidate(
                    parameter_bindings,
                    param_name,
                    candidate,
                );
                handled_rest_parameter = true;
                break;
            }
            let candidate = self.effective_parameter_value_binding_argument(
                argument,
                &current_function_value_bindings,
            );
            Self::merge_parameter_value_binding_candidate(
                parameter_bindings,
                param_name,
                candidate,
            );
        }

        if !handled_rest_parameter && call_arguments.len() < user_function.params.len() {
            for (index, param_name) in user_function
                .params
                .iter()
                .enumerate()
                .skip(call_arguments.len())
            {
                let candidate = if rest_parameter_index == Some(index) {
                    Some(Expression::Array(Vec::new()))
                } else {
                    None
                };
                Self::merge_parameter_value_binding_candidate(
                    parameter_bindings,
                    param_name,
                    candidate,
                );
            }
        }
    }

    fn merge_parameter_value_binding_candidate(
        parameter_bindings: &mut HashMap<String, Option<Expression>>,
        param_name: &str,
        candidate: Option<Expression>,
    ) {
        let Some(candidate) = candidate else {
            parameter_bindings.insert(param_name.to_string(), None);
            return;
        };
        match parameter_bindings.get(param_name) {
            Some(None) => {}
            Some(Some(existing)) if *existing == candidate => {}
            Some(Some(_)) => {
                parameter_bindings.insert(param_name.to_string(), None);
            }
            None => {
                parameter_bindings.insert(param_name.to_string(), Some(candidate));
            }
        }
    }

    fn effective_parameter_value_binding_argument(
        &self,
        argument: &Expression,
        current_function_value_bindings: &HashMap<String, Option<Expression>>,
    ) -> Option<Expression> {
        let materialized_argument = self.materialize_current_or_global_parameter_value_argument(
            argument,
            current_function_value_bindings,
        );
        let global_identifier_argument = matches!(
            argument,
            Expression::Identifier(name)
                if self.global_object_binding(name).is_some()
                    || self.global_array_binding(name).is_some()
        );
        let effective_argument = if global_identifier_argument {
            argument.clone()
        } else if matches!(
            argument,
            Expression::Member { property, .. }
                if matches!(property.as_ref(), Expression::String(name) if name == "prototype")
        ) {
            argument.clone()
        } else if matches!(
            materialized_argument,
            Expression::Member { ref property, .. }
                if matches!(property.as_ref(), Expression::String(name) if name == "prototype")
        ) {
            materialized_argument
        } else if matches!(
            materialized_argument,
            Expression::Number(_)
                | Expression::BigInt(_)
                | Expression::String(_)
                | Expression::Bool(_)
                | Expression::Null
                | Expression::Undefined
        ) {
            materialized_argument
        } else {
            self.infer_global_object_binding(argument)
                .map(|binding| object_binding_to_expression(&binding))
                .unwrap_or(materialized_argument)
        };
        (global_identifier_argument
            || self.prepared_parameter_value_argument_is_stable(&effective_argument))
        .then_some(effective_argument)
    }

    fn materialize_current_or_global_parameter_value_argument(
        &self,
        argument: &Expression,
        current_function_value_bindings: &HashMap<String, Option<Expression>>,
    ) -> Expression {
        let current_bindings = current_function_value_bindings
            .iter()
            .filter_map(|(name, binding)| {
                let binding = binding.as_ref()?;
                (!matches!(binding, Expression::Identifier(alias) if alias == name))
                    .then(|| (name.clone(), binding.clone()))
            })
            .collect::<HashMap<_, _>>();
        let substituted_argument = if current_bindings.is_empty() {
            argument.clone()
        } else {
            self.substitute_global_expression_bindings(argument, &current_bindings)
        };

        if let Some(text) = self.resolve_static_parameter_string_value(&substituted_argument) {
            return Expression::String(text);
        }

        let materialized =
            self.with_cloned_global_binding_state(|value_bindings, object_bindings| {
                self.materialize_global_expression_with_state(
                    &substituted_argument,
                    &HashMap::new(),
                    value_bindings,
                    object_bindings,
                )
                .unwrap_or_else(|| self.materialize_global_expression(&substituted_argument))
            });

        if let Some(text) = self.resolve_static_parameter_string_value(&materialized) {
            Expression::String(text)
        } else {
            materialized
        }
    }

    fn resolve_static_parameter_string_value(&self, expression: &Expression) -> Option<String> {
        self.resolve_static_parameter_string_value_inner(expression, &mut HashSet::new())
    }

    fn resolve_static_parameter_string_value_inner(
        &self,
        expression: &Expression,
        visited_identifiers: &mut HashSet<String>,
    ) -> Option<String> {
        match expression {
            Expression::String(text) => Some(text.clone()),
            Expression::Identifier(name) => {
                if !visited_identifiers.insert(name.clone()) {
                    return None;
                }
                let result = self.global_value_binding(name).and_then(|value| {
                    self.resolve_static_parameter_string_value_inner(value, visited_identifiers)
                });
                visited_identifiers.remove(name);
                result
            }
            Expression::Binary {
                op: BinaryOp::Add,
                left,
                right,
            } => {
                let mut left_visited = visited_identifiers.clone();
                let left_is_string = self
                    .resolve_static_parameter_string_value_inner(left, &mut left_visited)
                    .is_some();
                let mut right_visited = visited_identifiers.clone();
                let right_is_string = self
                    .resolve_static_parameter_string_value_inner(right, &mut right_visited)
                    .is_some();
                if !left_is_string && !right_is_string {
                    return None;
                }
                Some(format!(
                    "{}{}",
                    self.resolve_static_parameter_to_string_value_inner(left, visited_identifiers)?,
                    self.resolve_static_parameter_to_string_value_inner(
                        right,
                        visited_identifiers
                    )?
                ))
            }
            Expression::Member { object, property } => {
                let materialized_property = self.materialize_global_expression(property);
                let object_binding = self.infer_global_object_binding(object)?;
                object_binding_lookup_value(&object_binding, &materialized_property).and_then(
                    |value| {
                        self.resolve_static_parameter_string_value_inner(value, visited_identifiers)
                    },
                )
            }
            Expression::Call { callee, arguments } => {
                if arguments.is_empty()
                    && let Expression::Member { object, property } = callee.as_ref()
                    && matches!(property.as_ref(), Expression::String(name) if name == "toString")
                {
                    return self.resolve_static_parameter_symbol_to_string_value_inner(
                        object,
                        visited_identifiers,
                    );
                }
                None
            }
            _ => None,
        }
    }

    fn resolve_static_parameter_to_string_value_inner(
        &self,
        expression: &Expression,
        visited_identifiers: &mut HashSet<String>,
    ) -> Option<String> {
        match expression {
            Expression::String(text) => Some(text.clone()),
            Expression::Number(value) => Some(value.to_string()),
            Expression::Bool(value) => Some(value.to_string()),
            Expression::Null => Some("null".to_string()),
            Expression::Undefined => Some("undefined".to_string()),
            Expression::BigInt(value) => Some(value.clone()),
            Expression::Identifier(name) => {
                if !visited_identifiers.insert(name.clone()) {
                    return None;
                }
                let result = self.global_value_binding(name).and_then(|value| {
                    self.resolve_static_parameter_to_string_value_inner(value, visited_identifiers)
                });
                visited_identifiers.remove(name);
                result
            }
            Expression::Binary {
                op: BinaryOp::Add, ..
            } => self.resolve_static_parameter_string_value_inner(expression, visited_identifiers),
            Expression::Member { object, property } => {
                let materialized_property = self.materialize_global_expression(property);
                let object_binding = self.infer_global_object_binding(object)?;
                object_binding_lookup_value(&object_binding, &materialized_property).and_then(
                    |value| {
                        self.resolve_static_parameter_to_string_value_inner(
                            value,
                            visited_identifiers,
                        )
                    },
                )
            }
            Expression::Call { callee, arguments } => {
                if arguments.is_empty()
                    && let Expression::Member { object, property } = callee.as_ref()
                    && matches!(property.as_ref(), Expression::String(name) if name == "toString")
                {
                    return self.resolve_static_parameter_symbol_to_string_value_inner(
                        object,
                        visited_identifiers,
                    );
                }
                None
            }
            _ => None,
        }
    }

    fn resolve_static_parameter_symbol_to_string_value(
        &self,
        expression: &Expression,
    ) -> Option<String> {
        self.resolve_static_parameter_symbol_to_string_value_inner(expression, &mut HashSet::new())
    }

    fn resolve_static_parameter_symbol_to_string_value_inner(
        &self,
        expression: &Expression,
        visited_identifiers: &mut HashSet<String>,
    ) -> Option<String> {
        if let Expression::Identifier(name) = expression
            && let Some(value) = self.global_value_binding(name)
        {
            if !visited_identifiers.insert(name.clone()) {
                return None;
            }
            let result = self
                .resolve_static_parameter_symbol_to_string_value_inner(value, visited_identifiers);
            visited_identifiers.remove(name);
            return result;
        }

        if let Expression::Member { object, property } = expression {
            let materialized_property = self.materialize_global_expression(property);
            if let Some(object_binding) = self.infer_global_object_binding(object)
                && let Some(value) =
                    object_binding_lookup_value(&object_binding, &materialized_property)
            {
                return self.resolve_static_parameter_symbol_to_string_value_inner(
                    value,
                    visited_identifiers,
                );
            }
        }

        let Expression::Call { callee, arguments } = expression else {
            return None;
        };
        if !matches!(callee.as_ref(), Expression::Identifier(name) if name == "Symbol") {
            return None;
        }

        let description = match arguments.first() {
            None => String::new(),
            Some(
                CallArgument::Expression(Expression::Undefined)
                | CallArgument::Spread(Expression::Undefined),
            ) => String::new(),
            Some(CallArgument::Expression(argument) | CallArgument::Spread(argument)) => {
                self.resolve_static_parameter_string_value_inner(argument, visited_identifiers)?
            }
        };

        Some(format!("Symbol({description})"))
    }

    pub(in crate::backend::direct_wasm) fn prepared_parameter_argument_is_stable(
        argument: &Expression,
    ) -> bool {
        match argument {
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined => true,
            Expression::Array(elements) => elements.iter().all(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                    Self::prepared_parameter_argument_is_stable(expression)
                }
            }),
            Expression::Object(entries) => entries.iter().all(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    Self::prepared_parameter_argument_is_stable(key)
                        && Self::prepared_parameter_argument_is_stable(value)
                }
                ObjectEntry::Getter { key, getter } => {
                    Self::prepared_parameter_argument_is_stable(key)
                        && Self::prepared_parameter_argument_is_stable(getter)
                }
                ObjectEntry::Setter { key, setter } => {
                    Self::prepared_parameter_argument_is_stable(key)
                        && Self::prepared_parameter_argument_is_stable(setter)
                }
                ObjectEntry::Spread(expression) => {
                    Self::prepared_parameter_argument_is_stable(expression)
                }
            }),
            Expression::Member { object, property } => {
                matches!(property.as_ref(), Expression::String(name) if name == "prototype")
                    && !matches!(object.as_ref(), Expression::SuperMember { .. })
            }
            _ => false,
        }
    }

    fn prepared_parameter_value_argument_is_stable(&self, argument: &Expression) -> bool {
        match argument {
            Expression::Identifier(name) => {
                self.global_value_binding(name).is_some()
                    || self.global_object_binding(name).is_some()
                    || self.global_array_binding(name).is_some()
            }
            Expression::Unary { op, expression } => {
                matches!(
                    op,
                    UnaryOp::Negate
                        | UnaryOp::Plus
                        | UnaryOp::Not
                        | UnaryOp::BitwiseNot
                        | UnaryOp::Void
                ) && self.prepared_parameter_value_argument_is_stable(expression)
            }
            Expression::Call { callee, arguments } if matches!(callee.as_ref(), Expression::Identifier(name) if name == "Symbol") => {
                arguments.iter().all(|argument| {
                    self.prepared_parameter_value_argument_is_stable(argument.expression())
                })
            }
            Expression::Array(elements) => elements.iter().all(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                    self.prepared_parameter_value_argument_is_stable(expression)
                }
            }),
            Expression::Object(entries) => entries.iter().all(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    self.prepared_parameter_value_argument_is_stable(key)
                        && self.prepared_parameter_value_argument_is_stable(value)
                }
                ObjectEntry::Getter { key, getter } => {
                    self.prepared_parameter_value_argument_is_stable(key)
                        && self.prepared_parameter_value_argument_is_stable(getter)
                }
                ObjectEntry::Setter { key, setter } => {
                    self.prepared_parameter_value_argument_is_stable(key)
                        && self.prepared_parameter_value_argument_is_stable(setter)
                }
                ObjectEntry::Spread(expression) => {
                    self.prepared_parameter_value_argument_is_stable(expression)
                }
            }),
            _ => Self::prepared_parameter_argument_is_stable(argument),
        }
    }
}
