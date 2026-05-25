use super::*;

thread_local! {
    static ACTIVE_STATIC_LOCAL_ALIAS_NORMALIZATIONS:
        std::cell::RefCell<std::collections::HashSet<String>> =
            std::cell::RefCell::new(std::collections::HashSet::new());
}

struct StaticLocalAliasNormalizationGuard {
    key: String,
}

impl StaticLocalAliasNormalizationGuard {
    fn enter(expression: &Expression, current_function_name: Option<&str>) -> Option<Self> {
        let key = format!("{current_function_name:?}:{expression:?}");
        ACTIVE_STATIC_LOCAL_ALIAS_NORMALIZATIONS.with(|active| {
            let mut active = active.borrow_mut();
            if !active.insert(key.clone()) {
                return None;
            }
            Some(Self { key })
        })
    }
}

impl Drop for StaticLocalAliasNormalizationGuard {
    fn drop(&mut self) {
        ACTIVE_STATIC_LOCAL_ALIAS_NORMALIZATIONS.with(|active| {
            active.borrow_mut().remove(&self.key);
        });
    }
}

impl<'a> FunctionCompiler<'a> {
    fn normalize_returned_object_binding_from_return_shape(
        &self,
        mut object_binding: ObjectValueBinding,
        function_name: &str,
    ) -> ObjectValueBinding {
        let Some(function) = self.resolve_registered_function_declaration(function_name) else {
            return object_binding;
        };
        let Some(Statement::Return(Expression::Object(entries))) = function.body.last() else {
            return object_binding;
        };
        let mut replacements = Vec::new();
        for entry in entries {
            let ObjectEntry::Data {
                key,
                value: Expression::Identifier(source_name),
            } = entry
            else {
                continue;
            };
            let source_name = scoped_binding_source_name(source_name).unwrap_or(source_name);
            let property = self.materialize_static_expression(key);
            let Some(existing_value) =
                object_binding_lookup_value(&object_binding, &property).cloned()
            else {
                continue;
            };
            let mut referenced_names = HashSet::new();
            collect_referenced_binding_names_from_expression(
                &existing_value,
                &mut referenced_names,
            );
            let existing_references_source = referenced_names.iter().any(|referenced_name| {
                scoped_binding_source_name(referenced_name).unwrap_or(referenced_name)
                    == source_name
            });
            let source_expression = Expression::Identifier(source_name.to_string());
            let materialized_source = self.materialize_static_expression(&source_expression);
            let materialized_existing = self.materialize_static_expression(&existing_value);
            if existing_references_source
                || static_expression_matches(&existing_value, &materialized_source)
                || static_expression_matches(&materialized_existing, &materialized_source)
            {
                replacements.push((property, source_expression));
            }
        }
        for (property, value) in replacements {
            object_binding_set_property(&mut object_binding, property, value);
        }
        object_binding
    }

    fn normalize_returned_object_binding_after_runtime_user_call(
        &self,
        mut object_binding: ObjectValueBinding,
        function_name: &str,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> ObjectValueBinding {
        let expanded_arguments = self.expand_call_arguments(arguments);
        let this_binding = match callee {
            Expression::Member { object, .. } => self.materialize_static_expression(object),
            _ => Expression::Undefined,
        };
        let Some((_, updated_bindings)) = self
            .resolve_bound_snapshot_user_function_result_with_arguments_and_this(
                function_name,
                &HashMap::new(),
                &expanded_arguments,
                &this_binding,
            )
        else {
            return self.normalize_returned_object_binding_from_return_shape(
                object_binding,
                function_name,
            );
        };
        let trace_inherited_bindings = std::env::var_os("AYY_TRACE_INHERITED_BINDINGS").is_some();
        if trace_inherited_bindings {
            eprintln!(
                "normalize_returned_object_binding_after_runtime_user_call function={function_name} updates={updated_bindings:?} props={:?}",
                object_binding_to_expression(&object_binding)
            );
        }
        for (_, value) in &mut object_binding.string_properties {
            *value = self
                .replace_call_snapshot_updated_values_with_runtime_reads(value, &updated_bindings);
        }
        for (property, value) in &mut object_binding.symbol_properties {
            *property = self.replace_call_snapshot_updated_values_with_runtime_reads(
                property,
                &updated_bindings,
            );
            *value = self
                .replace_call_snapshot_updated_values_with_runtime_reads(value, &updated_bindings);
        }
        for (property, descriptor) in &mut object_binding.property_descriptors {
            *property = self.replace_call_snapshot_updated_values_with_runtime_reads(
                property,
                &updated_bindings,
            );
            if let Some(value) = descriptor.value.as_mut() {
                *value = self.replace_call_snapshot_updated_values_with_runtime_reads(
                    value,
                    &updated_bindings,
                );
            }
            if let Some(getter) = descriptor.getter.as_mut() {
                *getter = self.replace_call_snapshot_updated_values_with_runtime_reads(
                    getter,
                    &updated_bindings,
                );
            }
            if let Some(setter) = descriptor.setter.as_mut() {
                *setter = self.replace_call_snapshot_updated_values_with_runtime_reads(
                    setter,
                    &updated_bindings,
                );
            }
        }
        if trace_inherited_bindings {
            eprintln!(
                "normalize_returned_object_binding_after_runtime_user_call normalized_props={:?}",
                object_binding_to_expression(&object_binding)
            );
        }
        self.normalize_returned_object_binding_from_return_shape(object_binding, function_name)
    }

    fn static_member_is_restricted_function_property(
        &self,
        object: &Expression,
        property: &Expression,
        environment: &StaticResolutionEnvironment,
    ) -> bool {
        let mut property_environment = environment.fork();
        let property = self
            .evaluate_static_expression_with_state(property, &mut property_environment)
            .or_else(|| {
                self.materialize_static_expression_with_state(property, &property_environment)
            })
            .unwrap_or_else(|| property.clone());
        if !matches!(
            property,
            Expression::String(ref property_name)
                if property_name == "caller" || property_name == "arguments"
        ) {
            return false;
        }

        if self.is_restricted_function_property(object, &property) {
            return true;
        }
        if let Some(resolved_object) = self
            .resolve_bound_alias_expression_with_state(object, environment)
            .filter(|resolved| !static_expression_matches(resolved, object))
            && self.is_restricted_function_property(&resolved_object, &property)
        {
            return true;
        }
        let mut object_environment = environment.fork();
        if let Some(evaluated_object) =
            self.evaluate_static_expression_with_state(object, &mut object_environment)
            && self.is_restricted_function_property(&evaluated_object, &property)
        {
            return true;
        }
        if let Some(materialized_object) =
            self.materialize_static_expression_with_state(object, environment)
        {
            return self.is_restricted_function_property(&materialized_object, &property);
        }
        false
    }

    fn static_expression_may_read_restricted_function_property(
        &self,
        expression: &Expression,
        environment: &StaticResolutionEnvironment,
    ) -> bool {
        match expression {
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => self
                    .static_expression_may_read_restricted_function_property(
                        expression,
                        environment,
                    ),
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    self.static_expression_may_read_restricted_function_property(key, environment)
                        || self.static_expression_may_read_restricted_function_property(
                            value,
                            environment,
                        )
                }
                ObjectEntry::Getter { key, getter } => {
                    self.static_expression_may_read_restricted_function_property(key, environment)
                        || self.static_expression_may_read_restricted_function_property(
                            getter,
                            environment,
                        )
                }
                ObjectEntry::Setter { key, setter } => {
                    self.static_expression_may_read_restricted_function_property(key, environment)
                        || self.static_expression_may_read_restricted_function_property(
                            setter,
                            environment,
                        )
                }
                ObjectEntry::Spread(expression) => self
                    .static_expression_may_read_restricted_function_property(
                        expression,
                        environment,
                    ),
            }),
            Expression::Member { object, property } => {
                self.static_member_is_restricted_function_property(object, property, environment)
                    || self.static_expression_may_read_restricted_function_property(
                        object,
                        environment,
                    )
                    || self.static_expression_may_read_restricted_function_property(
                        property,
                        environment,
                    )
            }
            Expression::SuperMember { property } => {
                self.static_expression_may_read_restricted_function_property(property, environment)
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => self.static_expression_may_read_restricted_function_property(value, environment),
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.static_expression_may_read_restricted_function_property(object, environment)
                    || self.static_expression_may_read_restricted_function_property(
                        property,
                        environment,
                    )
                    || self
                        .static_expression_may_read_restricted_function_property(value, environment)
            }
            Expression::AssignSuperMember { property, value } => {
                self.static_expression_may_read_restricted_function_property(property, environment)
                    || self
                        .static_expression_may_read_restricted_function_property(value, environment)
            }
            Expression::Binary { left, right, .. } => {
                self.static_expression_may_read_restricted_function_property(left, environment)
                    || self
                        .static_expression_may_read_restricted_function_property(right, environment)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.static_expression_may_read_restricted_function_property(condition, environment)
                    || self.static_expression_may_read_restricted_function_property(
                        then_expression,
                        environment,
                    )
                    || self.static_expression_may_read_restricted_function_property(
                        else_expression,
                        environment,
                    )
            }
            Expression::Sequence(expressions) => expressions.iter().any(|expression| {
                self.static_expression_may_read_restricted_function_property(
                    expression,
                    environment,
                )
            }),
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                self.static_expression_may_read_restricted_function_property(callee, environment)
                    || arguments.iter().any(|argument| {
                        self.static_expression_may_read_restricted_function_property(
                            argument.expression(),
                            environment,
                        )
                    })
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

    fn static_statement_may_read_restricted_function_property(
        &self,
        statement: &Statement,
        environment: &StaticResolutionEnvironment,
    ) -> bool {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. } => {
                self.static_statements_may_read_restricted_function_property(body, environment)
            }
            Statement::Var { value, .. }
            | Statement::Let { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Expression(value)
            | Statement::Throw(value)
            | Statement::Return(value)
            | Statement::Yield { value }
            | Statement::YieldDelegate { value } => {
                self.static_expression_may_read_restricted_function_property(value, environment)
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                self.static_expression_may_read_restricted_function_property(object, environment)
                    || self.static_expression_may_read_restricted_function_property(
                        property,
                        environment,
                    )
                    || self
                        .static_expression_may_read_restricted_function_property(value, environment)
            }
            Statement::Print { values } => values.iter().any(|value| {
                self.static_expression_may_read_restricted_function_property(value, environment)
            }),
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.static_expression_may_read_restricted_function_property(condition, environment)
                    || self.static_statements_may_read_restricted_function_property(
                        then_branch,
                        environment,
                    )
                    || self.static_statements_may_read_restricted_function_property(
                        else_branch,
                        environment,
                    )
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                self.static_statements_may_read_restricted_function_property(body, environment)
                    || self.static_statements_may_read_restricted_function_property(
                        catch_setup,
                        environment,
                    )
                    || self.static_statements_may_read_restricted_function_property(
                        catch_body,
                        environment,
                    )
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                self.static_expression_may_read_restricted_function_property(
                    discriminant,
                    environment,
                ) || cases.iter().any(|case| {
                    case.test.as_ref().is_some_and(|test| {
                        self.static_expression_may_read_restricted_function_property(
                            test,
                            environment,
                        )
                    }) || self.static_statements_may_read_restricted_function_property(
                        &case.body,
                        environment,
                    )
                })
            }
            Statement::For {
                init,
                condition,
                update,
                break_hook,
                body,
                ..
            } => {
                self.static_statements_may_read_restricted_function_property(init, environment)
                    || condition.as_ref().is_some_and(|condition| {
                        self.static_expression_may_read_restricted_function_property(
                            condition,
                            environment,
                        )
                    })
                    || update.as_ref().is_some_and(|update| {
                        self.static_expression_may_read_restricted_function_property(
                            update,
                            environment,
                        )
                    })
                    || break_hook.as_ref().is_some_and(|break_hook| {
                        self.static_expression_may_read_restricted_function_property(
                            break_hook,
                            environment,
                        )
                    })
                    || self
                        .static_statements_may_read_restricted_function_property(body, environment)
            }
            Statement::While {
                condition,
                break_hook,
                body,
                ..
            }
            | Statement::DoWhile {
                condition,
                break_hook,
                body,
                ..
            } => {
                self.static_expression_may_read_restricted_function_property(condition, environment)
                    || break_hook.as_ref().is_some_and(|break_hook| {
                        self.static_expression_may_read_restricted_function_property(
                            break_hook,
                            environment,
                        )
                    })
                    || self
                        .static_statements_may_read_restricted_function_property(body, environment)
            }
            Statement::Break { .. } | Statement::Continue { .. } => false,
        }
    }

    fn static_statements_may_read_restricted_function_property(
        &self,
        statements: &[Statement],
        environment: &StaticResolutionEnvironment,
    ) -> bool {
        statements.iter().any(|statement| {
            self.static_statement_may_read_restricted_function_property(statement, environment)
        })
    }

    fn materialize_static_return_object_binding_expression_with_state(
        &self,
        expression: &Expression,
        environment: &mut StaticResolutionEnvironment,
    ) -> Expression {
        if let Some(resolved) = self
            .resolve_bound_alias_expression_with_state(expression, environment)
            .filter(|resolved| !static_expression_matches(resolved, expression))
        {
            return self.materialize_static_return_object_binding_expression_with_state(
                &resolved,
                environment,
            );
        }
        self.evaluate_static_expression_with_state(expression, environment)
            .or_else(|| self.materialize_static_expression_with_state(expression, environment))
            .or_else(|| {
                materialize_recursive_expression(expression, true, true, &|nested| {
                    let mut nested_environment = environment.clone();
                    Some(
                        self.materialize_static_return_object_binding_expression_with_state(
                            nested,
                            &mut nested_environment,
                        ),
                    )
                })
            })
            .unwrap_or_else(|| expression.clone())
    }

    fn normalize_static_class_constructor_alias_expression(
        &self,
        expression: &Expression,
    ) -> Expression {
        match expression {
            Expression::Identifier(name) => self
                .resolve_static_class_init_constructor_alias(name)
                .map(Expression::Identifier)
                .unwrap_or_else(|| expression.clone()),
            Expression::Member { object, property } => Expression::Member {
                object: Box::new(self.normalize_static_class_constructor_alias_expression(object)),
                property: Box::new(
                    self.normalize_static_class_constructor_alias_expression(property),
                ),
            },
            Expression::Call { callee, arguments } => Expression::Call {
                callee: Box::new(self.normalize_static_class_constructor_alias_expression(callee)),
                arguments: arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(expression) => CallArgument::Expression(
                            self.normalize_static_class_constructor_alias_expression(expression),
                        ),
                        CallArgument::Spread(expression) => CallArgument::Spread(
                            self.normalize_static_class_constructor_alias_expression(expression),
                        ),
                    })
                    .collect(),
            },
            Expression::New { callee, arguments } => Expression::New {
                callee: Box::new(self.normalize_static_class_constructor_alias_expression(callee)),
                arguments: arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(expression) => CallArgument::Expression(
                            self.normalize_static_class_constructor_alias_expression(expression),
                        ),
                        CallArgument::Spread(expression) => CallArgument::Spread(
                            self.normalize_static_class_constructor_alias_expression(expression),
                        ),
                    })
                    .collect(),
            },
            _ => expression.clone(),
        }
    }

    fn normalize_static_local_alias_expression(
        &self,
        expression: &Expression,
        aliases: &HashMap<String, Expression>,
    ) -> Expression {
        let Some(_guard) =
            StaticLocalAliasNormalizationGuard::enter(expression, self.current_function_name())
        else {
            return expression.clone();
        };
        let resolved = resolve_returned_member_local_alias_expression(expression, aliases);
        if !static_expression_matches(&resolved, expression) {
            return self.normalize_static_local_alias_expression(&resolved, aliases);
        }
        materialize_recursive_expression(expression, true, true, &|nested| {
            Some(self.normalize_static_local_alias_expression(nested, aliases))
        })
        .unwrap_or_else(|| expression.clone())
    }

    fn static_function_constructor_return_expression(
        &self,
        function_name: &str,
    ) -> Option<Expression> {
        if !function_name.starts_with("__ayy_function_ctor_") {
            return None;
        }
        let function = self.resolve_registered_function_declaration(function_name)?;
        let [Statement::Return(return_value)] = function.body.as_slice() else {
            return None;
        };
        Some(return_value.clone())
    }

    fn normalize_static_function_constructor_alias_expression(
        &self,
        expression: &Expression,
        aliases: &HashMap<String, Expression>,
    ) -> Expression {
        match expression {
            Expression::Identifier(name) => aliases
                .get(name)
                .cloned()
                .unwrap_or_else(|| expression.clone()),
            Expression::Member { object, property } => Expression::Member {
                object: Box::new(
                    self.normalize_static_function_constructor_alias_expression(object, aliases),
                ),
                property: Box::new(
                    self.normalize_static_function_constructor_alias_expression(property, aliases),
                ),
            },
            Expression::Call { callee, arguments } => {
                let callee =
                    self.normalize_static_function_constructor_alias_expression(callee, aliases);
                let arguments = arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(expression) => CallArgument::Expression(
                            self.normalize_static_function_constructor_alias_expression(
                                expression, aliases,
                            ),
                        ),
                        CallArgument::Spread(expression) => CallArgument::Spread(
                            self.normalize_static_function_constructor_alias_expression(
                                expression, aliases,
                            ),
                        ),
                    })
                    .collect::<Vec<_>>();
                if arguments.is_empty()
                    && let Expression::Identifier(function_name) = &callee
                    && let Some(return_value) =
                        self.static_function_constructor_return_expression(function_name)
                {
                    return self.normalize_static_function_constructor_alias_expression(
                        &return_value,
                        aliases,
                    );
                }
                Expression::Call {
                    callee: Box::new(callee),
                    arguments,
                }
            }
            Expression::New { callee, arguments } => Expression::New {
                callee: Box::new(
                    self.normalize_static_function_constructor_alias_expression(callee, aliases),
                ),
                arguments: arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(expression) => CallArgument::Expression(
                            self.normalize_static_function_constructor_alias_expression(
                                expression, aliases,
                            ),
                        ),
                        CallArgument::Spread(expression) => CallArgument::Spread(
                            self.normalize_static_function_constructor_alias_expression(
                                expression, aliases,
                            ),
                        ),
                    })
                    .collect(),
            },
            _ => materialize_recursive_expression(expression, true, true, &|nested| {
                Some(self.normalize_static_function_constructor_alias_expression(nested, aliases))
            })
            .unwrap_or_else(|| expression.clone()),
        }
    }

    fn static_function_constructor_construct_return_expression_from_body(
        &self,
        statements: &[Statement],
    ) -> Option<Expression> {
        let mut aliases = HashMap::new();
        for statement in statements {
            match statement {
                Statement::Var { name, value }
                | Statement::Let { name, value, .. }
                | Statement::Assign { name, value } => {
                    let normalized = self
                        .normalize_static_function_constructor_alias_expression(value, &aliases);
                    aliases.insert(name.clone(), normalized);
                }
                Statement::Declaration { body } | Statement::Block { body } => {
                    if let Some(return_value) =
                        self.static_function_constructor_construct_return_expression_from_body(body)
                    {
                        return Some(return_value);
                    }
                }
                Statement::Return(return_value) => {
                    let normalized = self.normalize_static_function_constructor_alias_expression(
                        return_value,
                        &aliases,
                    );
                    if matches!(normalized, Expression::New { .. }) {
                        return Some(normalized);
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn collect_static_local_private_constructor_marker_bindings_from_define_property_call(
        &self,
        arguments: &[CallArgument],
        environment: &mut StaticResolutionEnvironment,
        markers_by_constructor: &mut HashMap<String, ObjectValueBinding>,
    ) {
        let [
            CallArgument::Expression(target_expression),
            CallArgument::Expression(property_expression),
            CallArgument::Expression(descriptor_expression),
            ..,
        ] = arguments
        else {
            return;
        };
        let Some(descriptor) = resolve_property_descriptor_definition(descriptor_expression) else {
            return;
        };
        let property = self.materialize_static_return_object_binding_expression_with_state(
            property_expression,
            environment,
        );
        let Some(property_name) = static_property_name_from_expression(&property) else {
            return;
        };
        if !property_name.starts_with("__ayy$private$") {
            return;
        }
        let target = self.materialize_static_return_object_binding_expression_with_state(
            target_expression,
            environment,
        );
        let Expression::Member { object, property } = target else {
            return;
        };
        if !matches!(
            property.as_ref(),
            Expression::String(name) if name == "prototype"
        ) {
            return;
        }
        let Expression::Identifier(constructor_name) = object.as_ref() else {
            return;
        };
        let value_expression = descriptor
            .getter
            .as_ref()
            .or(descriptor.value.as_ref())
            .or(descriptor.setter.as_ref())
            .map(|value| {
                let value = self.materialize_static_return_object_binding_expression_with_state(
                    value,
                    environment,
                );
                self.resolve_function_binding_from_expression(&value)
                    .and_then(|binding| match binding {
                        LocalFunctionBinding::User(function_name) => self
                            .user_function(&function_name)
                            .and_then(|function| function.private_brand_binding.clone())
                            .map(|binding_name| {
                                self.materialize_static_return_object_binding_expression_with_state(
                                    &Expression::Identifier(binding_name),
                                    environment,
                                )
                            }),
                        LocalFunctionBinding::Builtin(_) => None,
                    })
                    .unwrap_or(value)
            })
            .unwrap_or(Expression::Undefined);
        let enumerable = descriptor.enumerable.unwrap_or(false);
        let object_binding = markers_by_constructor
            .entry(constructor_name.clone())
            .or_insert_with(empty_object_value_binding);
        object_binding_define_property(
            object_binding,
            Expression::String(property_name),
            value_expression,
            enumerable,
        );
    }

    fn collect_static_local_private_constructor_marker_bindings_from_statements(
        &self,
        statements: &[Statement],
        environment: &mut StaticResolutionEnvironment,
        markers_by_constructor: &mut HashMap<String, ObjectValueBinding>,
    ) {
        for statement in statements {
            match statement {
                Statement::Var { name, value } | Statement::Let { name, value, .. } => {
                    let value = self
                        .materialize_static_return_object_binding_expression_with_state(
                            value,
                            environment,
                        );
                    environment.set_local_binding(name.clone(), value);
                }
                Statement::Assign { name, value } => {
                    let value = self
                        .materialize_static_return_object_binding_expression_with_state(
                            value,
                            environment,
                        );
                    environment.assign_binding_value(name.clone(), value);
                }
                Statement::Declaration { body } | Statement::Block { body } => {
                    self.collect_static_local_private_constructor_marker_bindings_from_statements(
                        body,
                        environment,
                        markers_by_constructor,
                    );
                }
                Statement::Expression(Expression::Call { callee, arguments })
                | Statement::Return(Expression::Call { callee, arguments })
                    if matches!(
                        callee.as_ref(),
                        Expression::Member { object, property }
                            if matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
                                && matches!(property.as_ref(), Expression::String(name) if name == "defineProperty")
                    ) =>
                {
                    self.collect_static_local_private_constructor_marker_bindings_from_define_property_call(
                        arguments,
                        environment,
                        markers_by_constructor,
                    );
                }
                _ => {}
            }
        }
    }

    fn collect_static_local_private_constructor_marker_bindings(
        &self,
        statements: &[Statement],
        environment: &StaticResolutionEnvironment,
    ) -> HashMap<String, ObjectValueBinding> {
        let mut environment = environment.fork();
        let mut markers_by_constructor = HashMap::new();
        self.collect_static_local_private_constructor_marker_bindings_from_statements(
            statements,
            &mut environment,
            &mut markers_by_constructor,
        );
        markers_by_constructor
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_return_expression_from_user_function_call(
        &self,
        function_name: &str,
        arguments: &[CallArgument],
        capture_source_bindings: Option<&HashMap<String, Expression>>,
    ) -> Option<Expression> {
        let user_function = self.user_function(function_name)?;
        let function = self.resolve_registered_function_declaration(function_name)?;
        if function
            .body
            .iter()
            .take(function.body.len().saturating_sub(1))
            .any(Self::statement_unconditionally_transfers_control)
        {
            return None;
        }
        let mut execution = self.prepare_static_user_function_execution(
            function_name,
            user_function,
            arguments,
            &Expression::Undefined,
            capture_source_bindings,
            HashMap::new(),
            |statement| statement,
        )?;
        execution.substituted_body = execution
            .substituted_body
            .into_iter()
            .map(|statement| Self::sanitize_static_return_expression_statement(&statement))
            .collect();
        if self.static_statements_may_read_restricted_function_property(
            &execution.substituted_body,
            &execution.environment,
        ) {
            return None;
        }
        let local_aliases = collect_returned_member_local_aliases(&execution.substituted_body);
        if let Some(Some(return_value)) = self.execute_static_statements_with_state(
            &execution.substituted_body,
            &mut execution.environment,
        ) {
            let return_value = self.resolve_static_direct_eval_construct_return_expression(
                &return_value,
                function_name,
            );
            return Some(self.normalize_static_class_constructor_alias_expression(
                &self.normalize_static_local_alias_expression(
                    &self.materialize_static_return_object_binding_expression_with_state(
                        &return_value,
                        &mut execution.environment,
                    ),
                    &local_aliases,
                ),
            ));
        }
        let returned_expression = match function.body.last()? {
            Statement::Return(expression) => expression.clone(),
            _ => collect_returned_identifier_source_expression(&function.body)?,
        };
        let returned_expression = self.resolve_static_direct_eval_construct_return_expression(
            &returned_expression,
            function_name,
        );
        Some(self.normalize_static_class_constructor_alias_expression(
            &self.normalize_static_local_alias_expression(
                &self.materialize_static_return_object_binding_expression_with_state(
                    &returned_expression,
                    &mut execution.environment,
                ),
                &local_aliases,
            ),
        ))
    }

    fn merge_static_local_private_constructor_markers(
        &self,
        object_binding: &mut ObjectValueBinding,
        value: &Expression,
        environment: &mut StaticResolutionEnvironment,
        markers_by_constructor: &HashMap<String, ObjectValueBinding>,
    ) {
        let value =
            self.materialize_static_return_object_binding_expression_with_state(value, environment);
        let trace_inherited_bindings = std::env::var_os("AYY_TRACE_INHERITED_BINDINGS").is_some();
        let Expression::New { callee, .. } = value else {
            if trace_inherited_bindings {
                eprintln!("merge_static_local_private_constructor_markers:skip value={value:?}");
            }
            return;
        };
        let Expression::Identifier(constructor_name) = callee.as_ref() else {
            if trace_inherited_bindings {
                eprintln!(
                    "merge_static_local_private_constructor_markers:skip_callee callee={callee:?}"
                );
            }
            return;
        };
        let Some(marker_binding) = markers_by_constructor.get(constructor_name) else {
            if trace_inherited_bindings {
                eprintln!(
                    "merge_static_local_private_constructor_markers:no_markers constructor={} known={:?}",
                    constructor_name,
                    markers_by_constructor.keys().collect::<Vec<_>>(),
                );
            }
            return;
        };
        if trace_inherited_bindings {
            eprintln!(
                "merge_static_local_private_constructor_markers:merge constructor={} props={:?}",
                constructor_name,
                ordered_object_property_names(marker_binding),
            );
        }
        Self::merge_object_binding_properties(object_binding, marker_binding);
    }

    fn sanitize_static_return_expression_statement(statement: &Statement) -> Statement {
        match statement {
            Statement::Declaration { body } => Statement::Declaration {
                body: body
                    .iter()
                    .filter_map(|statement| match statement {
                        Statement::Var { .. }
                        | Statement::Let { .. }
                        | Statement::Assign { .. }
                        | Statement::Return(_)
                        | Statement::Throw(_) => {
                            Some(Self::sanitize_static_return_expression_statement(statement))
                        }
                        Statement::Declaration { .. } | Statement::Block { .. } => {
                            Some(Self::sanitize_static_return_expression_statement(statement))
                        }
                        _ => None,
                    })
                    .collect(),
            },
            Statement::Block { body } => Statement::Block {
                body: body
                    .iter()
                    .map(Self::sanitize_static_return_expression_statement)
                    .collect(),
            },
            _ => statement.clone(),
        }
    }

    fn static_return_object_binding_can_skip_expression_statement(expression: &Expression) -> bool {
        match expression {
            Expression::Call { callee, .. } => !matches!(
                callee.as_ref(),
                Expression::Member { object, property }
                    if matches!(object.as_ref(), Expression::Identifier(name) if name == "Object" || name == "Reflect")
                        && matches!(property.as_ref(), Expression::String(name) if name == "defineProperty")
            ),
            _ => false,
        }
    }

    fn sanitize_static_return_object_binding_statement(statement: &Statement) -> Statement {
        match statement {
            Statement::Expression(expression)
                if Self::static_return_object_binding_can_skip_expression_statement(expression) =>
            {
                Statement::Block { body: Vec::new() }
            }
            Statement::Declaration { body } => Statement::Declaration {
                body: body
                    .iter()
                    .filter_map(|statement| match statement {
                        Statement::Var { .. }
                        | Statement::Let { .. }
                        | Statement::Assign { .. }
                        | Statement::Return(_)
                        | Statement::Throw(_) => Some(
                            Self::sanitize_static_return_object_binding_statement(statement),
                        ),
                        Statement::Declaration { .. } | Statement::Block { .. } => Some(
                            Self::sanitize_static_return_object_binding_statement(statement),
                        ),
                        Statement::Expression(expression)
                            if Self::static_return_object_binding_can_skip_expression_statement(
                                expression,
                            ) =>
                        {
                            Some(Self::sanitize_static_return_object_binding_statement(
                                statement,
                            ))
                        }
                        _ => None,
                    })
                    .collect(),
            },
            Statement::Block { body } => Statement::Block {
                body: body
                    .iter()
                    .map(Self::sanitize_static_return_object_binding_statement)
                    .collect(),
            },
            _ => statement.clone(),
        }
    }

    fn resolve_returned_object_binding_alias_fallback_with_environment(
        &self,
        function_name: &str,
        environment: Option<&mut StaticResolutionEnvironment>,
    ) -> Option<ObjectValueBinding> {
        let function = self.resolve_registered_function_declaration(function_name)?;
        let local_aliases = collect_returned_member_local_aliases(&function.body);
        let returned_expression = match function.body.last()? {
            Statement::Return(expression) => expression.clone(),
            _ => collect_returned_identifier_source_expression(&function.body)?,
        };
        let resolved_expression = match returned_expression {
            Expression::Identifier(_) => {
                resolve_returned_member_local_alias_expression(&returned_expression, &local_aliases)
            }
            Expression::New { callee, arguments } => Expression::New {
                callee: Box::new(resolve_returned_member_local_alias_expression(
                    callee.as_ref(),
                    &local_aliases,
                )),
                arguments: arguments
                    .into_iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(expression) => CallArgument::Expression(
                            resolve_returned_member_local_alias_expression(
                                &expression,
                                &local_aliases,
                            ),
                        ),
                        CallArgument::Spread(expression) => {
                            CallArgument::Spread(resolve_returned_member_local_alias_expression(
                                &expression,
                                &local_aliases,
                            ))
                        }
                    })
                    .collect(),
            },
            Expression::Call { callee, arguments } => Expression::Call {
                callee: Box::new(resolve_returned_member_local_alias_expression(
                    callee.as_ref(),
                    &local_aliases,
                )),
                arguments: arguments
                    .into_iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(expression) => CallArgument::Expression(
                            resolve_returned_member_local_alias_expression(
                                &expression,
                                &local_aliases,
                            ),
                        ),
                        CallArgument::Spread(expression) => {
                            CallArgument::Spread(resolve_returned_member_local_alias_expression(
                                &expression,
                                &local_aliases,
                            ))
                        }
                    })
                    .collect(),
            },
            _ => returned_expression,
        };
        let mut environment = environment;

        if let Some(environment) = environment.as_deref_mut() {
            if let Some(object_binding) = self.resolve_object_binding_from_expression_with_state(
                &resolved_expression,
                environment,
            ) {
                return Some(object_binding);
            }
            if let Some(materialized_expression) = self
                .materialize_static_expression_with_state(&resolved_expression, environment)
                .filter(|materialized| {
                    !static_expression_matches(materialized, &resolved_expression)
                })
                && let Some(object_binding) = self
                    .resolve_object_binding_from_expression_with_state(
                        &materialized_expression,
                        environment,
                    )
            {
                return Some(object_binding);
            }
        }

        self.resolve_object_binding_from_expression(&resolved_expression)
            .or_else(|| match resolved_expression {
                Expression::New { callee, arguments } => {
                    self.resolve_user_constructor_object_binding_from_new(&callee, &arguments)
                }
                Expression::Call { callee, arguments } => {
                    self.resolve_returned_object_binding_from_call(&callee, &arguments)
                }
                _ => None,
            })
    }

    pub(in crate::backend::direct_wasm) fn resolve_fresh_simple_generator_next_result_expression(
        &self,
        object: &Expression,
        arguments: &[CallArgument],
    ) -> Option<Expression> {
        let iter_result_object = |done: bool, value: Expression| {
            Expression::Object(vec![
                ObjectEntry::Data {
                    key: Expression::String("done".to_string()),
                    value: Expression::Bool(done),
                },
                ObjectEntry::Data {
                    key: Expression::String("value".to_string()),
                    value,
                },
            ])
        };
        let Some((is_async, steps, _, completion_value)) = self
            .simple_generator_source_metadata(object)
            .or_else(|| {
                self.resolve_simple_generator_source(object).map(
                    |(steps, completion_effects, completion_value)| {
                        let is_async = if let Expression::Call { callee, .. } = object
                            && let Some(LocalFunctionBinding::User(function_name)) =
                                self.resolve_function_binding_from_expression(callee)
                        {
                            self.user_function(&function_name).is_some_and(|function| {
                                matches!(function.kind, FunctionKind::AsyncGenerator)
                            })
                        } else {
                            false
                        };
                        (is_async, steps, completion_effects, completion_value)
                    },
                )
            })
            .or_else(|| {
                self.resolve_array_prototype_simple_generator_source(object)
                    .map(|(steps, completion_effects, completion_value)| {
                        (false, steps, completion_effects, completion_value)
                    })
            })
        else {
            return None;
        };
        if is_async {
            return None;
        }

        let current_index = match object {
            Expression::Identifier(object_name) => {
                let binding_name = self
                    .resolve_local_array_iterator_binding_name(object_name)
                    .unwrap_or_else(|| object_name.clone());
                self.state
                    .speculation
                    .static_semantics
                    .local_array_iterator_binding(&binding_name)
                    .and_then(|binding| binding.static_index)?
            }
            _ => 0,
        };
        let source_function_name = self.simple_generator_source_function_name(object);
        let sent_value = arguments
            .first()
            .map(|argument| match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    self.materialize_static_expression(expression)
                }
            })
            .unwrap_or(Expression::Undefined);

        if let Some(step) = steps.get(current_index) {
            return match &step.outcome {
                SimpleGeneratorStepOutcome::Yield(value) => {
                    let yielded_value = Self::substitute_sent_expression(value, &sent_value);
                    let yielded_value = self.resolve_simple_generator_result_value_with_context(
                        &yielded_value,
                        source_function_name.as_deref(),
                    );
                    Some(iter_result_object(false, yielded_value))
                }
                SimpleGeneratorStepOutcome::YieldResult(result) => Some(
                    self.materialize_static_expression(&Self::substitute_sent_expression(
                        result,
                        &sent_value,
                    )),
                ),
                SimpleGeneratorStepOutcome::Throw(_) => None,
            };
        }

        if current_index == steps.len() {
            return Some(iter_result_object(
                true,
                self.resolve_simple_generator_result_value_with_context(
                    &completion_value,
                    source_function_name.as_deref(),
                ),
            ));
        }

        Some(iter_result_object(true, Expression::Undefined))
    }

    pub(in crate::backend::direct_wasm) fn resolve_call_snapshot_result_expression(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        self.state
            .speculation
            .static_semantics
            .last_bound_user_function_call
            .as_ref()
            .filter(|snapshot| {
                snapshot
                    .source_expression
                    .as_ref()
                    .is_some_and(|source| static_expression_matches(source, expression))
            })
            .and_then(|snapshot| snapshot.result_expression.as_ref())
            .cloned()
    }

    pub(in crate::backend::direct_wasm) fn resolve_user_function_from_callee_name(
        &self,
        name: &str,
    ) -> Option<&UserFunction> {
        let resolved_name = self
            .resolve_current_local_binding(name)
            .map(|(resolved_name, _)| resolved_name)
            .unwrap_or_else(|| name.to_string());
        if let Some(LocalFunctionBinding::User(function_name)) = self
            .state
            .speculation
            .static_semantics
            .local_function_binding(&resolved_name)
        {
            return self
                .backend
                .function_registry
                .catalog
                .user_function(&function_name);
        }
        let global_value = self.global_value_binding(name);
        let global_value_is_self_reference = global_value.is_none_or(
            |value| matches!(value, Expression::Identifier(value_name) if value_name == name),
        );
        if self.global_has_binding(name)
            && global_value_is_self_reference
            && let Some(user_function) = self.user_function(name)
        {
            return Some(user_function);
        }
        self.resolve_user_function_by_binding_name(name)
    }

    pub(in crate::backend::direct_wasm) fn resolve_returned_member_value_from_expression(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<Expression> {
        let Expression::String(property_name) = property else {
            return None;
        };

        if let Some(snapshot_result) = self.resolve_call_snapshot_result_expression(object)
            && let Some(object_binding) =
                self.resolve_object_binding_from_expression(&snapshot_result)
            && let Some(value) = object_binding_lookup_value(
                &object_binding,
                &Expression::String(property_name.clone()),
            )
        {
            return Some(value.clone());
        }

        if matches!(property_name.as_str(), "done" | "value")
            && let Expression::Call { callee, arguments } = object
            && let Expression::Member {
                object: iterator_object,
                property: next_property,
            } = callee.as_ref()
            && matches!(next_property.as_ref(), Expression::String(name) if name == "next")
            && let Some(next_result) = self
                .resolve_fresh_simple_generator_next_result_expression(iterator_object, arguments)
            && let Some(object_binding) = self.resolve_object_binding_from_expression(&next_result)
            && let Some(value) = object_binding_lookup_value(
                &object_binding,
                &Expression::String(property_name.clone()),
            )
        {
            return Some(value.clone());
        }

        let (callee, arguments) = match object {
            Expression::Call { callee, arguments } | Expression::New { callee, arguments } => {
                (callee.as_ref(), arguments.as_slice())
            }
            _ => return None,
        };
        if let Some(object_binding) =
            self.resolve_returned_object_binding_from_call(callee, arguments)
            && let Some(value) = object_binding_lookup_value(
                &object_binding,
                &Expression::String(property_name.clone()),
            )
        {
            return Some(value.clone());
        }
        let LocalFunctionBinding::User(function_name) =
            self.resolve_function_binding_from_expression(callee)?
        else {
            return None;
        };
        let user_function = self.user_function(&function_name)?;
        let binding = user_function
            .returned_member_value_bindings
            .iter()
            .find(|binding| binding.property == *property_name)?;

        let mut value = self.substitute_user_function_argument_bindings(
            &binding.value,
            user_function,
            arguments,
        );
        if let Expression::Member { object, property } = callee
            && let Some(capture_slots) =
                self.resolve_member_function_capture_slots(object, property)
        {
            value = self.substitute_capture_slot_bindings(&value, &capture_slots);
        }

        Some(value)
    }

    pub(in crate::backend::direct_wasm) fn resolve_returned_object_binding_from_call(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Option<ObjectValueBinding> {
        let trace_inherited_bindings = std::env::var_os("AYY_TRACE_INHERITED_BINDINGS").is_some();
        if trace_inherited_bindings {
            eprintln!(
                "resolve_returned_object_binding_from_call:start callee={callee:?} argc={}",
                arguments.len()
            );
        }
        let LocalFunctionBinding::User(function_name) =
            self.resolve_function_binding_from_expression(callee)?
        else {
            return None;
        };
        if trace_inherited_bindings {
            eprintln!("resolve_returned_object_binding_from_call:function={function_name}");
        }
        let capture_source_bindings =
            self.resolve_function_expression_capture_slots(callee)
                .map(|capture_slots| {
                    capture_slots
                        .into_iter()
                        .map(|(capture_name, slot_name)| {
                            let snapshot = self.snapshot_bound_capture_slot_expression(&slot_name);
                            let source_expression = self
                                .resolve_capture_slot_static_source_expression(&slot_name)
                                .filter(|source| {
                                    self.resolve_object_binding_from_expression(source)
                                        .is_some_and(|binding| {
                                            !binding.property_descriptors.is_empty()
                                        })
                                })
                                .unwrap_or(snapshot);
                            (capture_name, source_expression)
                        })
                        .collect::<HashMap<_, _>>()
                });
        let call_expression = Expression::Call {
            callee: Box::new(callee.clone()),
            arguments: arguments.to_vec(),
        };
        if let Some(snapshot) = self
            .state
            .speculation
            .static_semantics
            .last_bound_user_function_call
            .as_ref()
            .filter(|snapshot| {
                snapshot.function_name == function_name
                    && snapshot.source_expression.as_ref().is_some_and(|source| {
                        let materialized_source = self.materialize_static_expression(source);
                        let materialized_call =
                            self.materialize_static_expression(&call_expression);
                        static_expression_matches(&materialized_source, &materialized_call)
                    })
            })
            && let Some(result) = snapshot.result_expression.as_ref()
        {
            let snapshot_result = match result {
                Expression::Identifier(_) | Expression::This => result.clone(),
                _ => self.materialize_static_expression(result),
            };
            let mut snapshot_environment = self
                .snapshot_static_resolution_environment_with_local_bindings(
                    snapshot.updated_bindings.clone(),
                );
            if let Some(object_binding) = self
                .resolve_object_binding_from_expression_with_state(
                    &snapshot_result,
                    &mut snapshot_environment,
                )
                .or_else(|| {
                    self.materialize_static_expression_with_state(
                        &snapshot_result,
                        &snapshot_environment,
                    )
                    .filter(|materialized| {
                        !static_expression_matches(materialized, &snapshot_result)
                    })
                    .and_then(|materialized| {
                        self.resolve_object_binding_from_expression_with_state(
                            &materialized,
                            &mut snapshot_environment,
                        )
                    })
                })
            {
                if trace_inherited_bindings {
                    eprintln!("resolve_returned_object_binding_from_call:snapshot_state");
                }
                return Some(object_binding);
            }
            let result = self.replace_call_snapshot_updated_values_with_runtime_reads(
                &snapshot_result,
                &snapshot.updated_bindings,
            );
            if let Some(object_binding) = self.resolve_object_binding_from_expression(&result) {
                if trace_inherited_bindings {
                    eprintln!("resolve_returned_object_binding_from_call:snapshot");
                }
                return Some(object_binding);
            }
        }
        if let Some(object_binding) = self
            .resolve_static_returned_object_binding_from_user_function_call_with_capture_sources(
                &function_name,
                arguments,
                capture_source_bindings.as_ref(),
            )
        {
            let object_binding = self.normalize_returned_object_binding_after_runtime_user_call(
                object_binding,
                &function_name,
                callee,
                arguments,
            );
            if trace_inherited_bindings {
                eprintln!("resolve_returned_object_binding_from_call:static_eval");
            }
            return Some(object_binding);
        }
        let user_function = self.user_function(&function_name)?;
        if user_function.returned_member_value_bindings.is_empty() {
            if trace_inherited_bindings {
                eprintln!("resolve_returned_object_binding_from_call:no_bindings");
            }
            return None;
        }
        let capture_bindings = match callee {
            Expression::Member { object, property } => self
                .resolve_member_function_capture_slots(object, property)
                .unwrap_or_default(),
            _ => BTreeMap::new(),
        };
        let mut object_binding = empty_object_value_binding();
        for binding in &user_function.returned_member_value_bindings {
            let mut value = self.substitute_user_function_argument_bindings(
                &binding.value,
                user_function,
                arguments,
            );
            if !capture_bindings.is_empty() {
                value = self.substitute_capture_slot_bindings(&value, &capture_bindings);
            }
            object_binding_set_property(
                &mut object_binding,
                Expression::String(binding.property.clone()),
                value,
            );
        }
        if trace_inherited_bindings {
            eprintln!("resolve_returned_object_binding_from_call:synthetic");
        }
        Some(
            self.normalize_returned_object_binding_after_runtime_user_call(
                object_binding,
                &function_name,
                callee,
                arguments,
            ),
        )
    }

    pub(in crate::backend::direct_wasm) fn resolve_returned_function_binding_from_call(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Option<LocalFunctionBinding> {
        let LocalFunctionBinding::User(function_name) =
            self.resolve_function_binding_from_expression(callee)?
        else {
            return None;
        };
        let user_function = self.user_function(&function_name)?;
        let function = self.resolve_registered_function_declaration(&function_name)?;
        let returned_expression = match function.body.last()? {
            Statement::Return(expression) => expression.clone(),
            _ => collect_returned_identifier_source_expression(&function.body)?,
        };
        let arguments_binding = Expression::Array(
            arguments
                .iter()
                .map(|argument| match argument {
                    CallArgument::Expression(expression) => {
                        ArrayElement::Expression(expression.clone())
                    }
                    CallArgument::Spread(expression) => ArrayElement::Spread(expression.clone()),
                })
                .collect(),
        );
        let substituted_expression = match callee {
            Expression::Member { object, .. } => self.substitute_user_function_call_frame_bindings(
                &returned_expression,
                user_function,
                arguments,
                object,
                &arguments_binding,
            ),
            Expression::SuperMember { .. } => self.substitute_user_function_call_frame_bindings(
                &returned_expression,
                user_function,
                arguments,
                &Expression::This,
                &arguments_binding,
            ),
            _ => self.substitute_user_function_argument_bindings(
                &returned_expression,
                user_function,
                arguments,
            ),
        };
        let normalized_expression =
            self.normalize_static_class_constructor_alias_expression(&substituted_expression);
        self.resolve_function_binding_from_expression(&normalized_expression)
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_returned_object_binding_from_user_function_call(
        &self,
        function_name: &str,
        arguments: &[CallArgument],
    ) -> Option<ObjectValueBinding> {
        self.resolve_static_returned_object_binding_from_user_function_call_with_capture_sources(
            function_name,
            arguments,
            None,
        )
    }

    fn resolve_static_returned_object_binding_from_user_function_call_with_capture_sources(
        &self,
        function_name: &str,
        arguments: &[CallArgument],
        capture_source_bindings: Option<&HashMap<String, Expression>>,
    ) -> Option<ObjectValueBinding> {
        let trace_inherited_bindings = std::env::var_os("AYY_TRACE_INHERITED_BINDINGS").is_some();
        if trace_inherited_bindings {
            eprintln!(
                "resolve_static_returned_object_binding_from_user_function_call:start function={function_name} argc={}",
                arguments.len(),
            );
        }
        if arguments.is_empty()
            && let Some(result_expression) = self
                .resolve_static_direct_eval_construct_return_expression_from_user_function(
                    function_name,
                )
            && let Some(object_binding) =
                self.resolve_object_binding_from_expression(&result_expression)
        {
            if trace_inherited_bindings {
                eprintln!(
                    "resolve_static_returned_object_binding_from_user_function_call:direct_eval_construct result={result_expression:?}",
                );
            }
            return Some(object_binding);
        }
        if arguments.is_empty()
            && let Some(function) = self.resolve_registered_function_declaration(function_name)
            && let Some(result_expression) = self
                .static_function_constructor_construct_return_expression_from_body(&function.body)
            && let Some(object_binding) =
                self.resolve_object_binding_from_expression(&result_expression)
        {
            if trace_inherited_bindings {
                eprintln!(
                    "resolve_static_returned_object_binding_from_user_function_call:function_ctor_construct result={result_expression:?}",
                );
            }
            return Some(object_binding);
        }
        let user_function = self.user_function(function_name)?;
        let mut execution = self.prepare_static_user_function_execution(
            function_name,
            user_function,
            arguments,
            &Expression::Undefined,
            capture_source_bindings,
            HashMap::new(),
            |statement| statement,
        )?;
        let local_private_constructor_markers = self
            .collect_static_local_private_constructor_marker_bindings(
                &execution.substituted_body,
                &execution.environment,
            );
        execution.substituted_body = execution
            .substituted_body
            .into_iter()
            .map(|statement| Self::sanitize_static_return_object_binding_statement(&statement))
            .collect();
        if trace_inherited_bindings {
            eprintln!(
                "resolve_static_returned_object_binding_from_user_function_call:prepared function={function_name} body_len={} local_C={:?} markers={:?} body={:?}",
                execution.substituted_body.len(),
                execution.environment.binding("C"),
                local_private_constructor_markers
                    .iter()
                    .map(|(name, binding)| format!(
                        "{name}:{:?}",
                        ordered_object_property_names(binding)
                    ))
                    .collect::<Vec<_>>(),
                execution.substituted_body,
            );
        }
        if let Some(Some(return_value)) = self.execute_static_statements_with_state(
            &execution.substituted_body,
            &mut execution.environment,
        ) {
            let return_value = self.resolve_static_direct_eval_construct_return_expression(
                &return_value,
                function_name,
            );
            if trace_inherited_bindings {
                eprintln!(
                    "resolve_static_returned_object_binding_from_user_function_call:return function={function_name} value={return_value:?} local_C={:?} has_object_C={}",
                    execution.environment.binding("C"),
                    execution.environment.object_binding("C").is_some(),
                );
            }
            return self
                .resolve_object_binding_from_expression_with_state(
                    &return_value,
                    &mut execution.environment,
                )
                .map(|mut object_binding| {
                    self.merge_static_local_private_constructor_markers(
                        &mut object_binding,
                        &return_value,
                        &mut execution.environment,
                        &local_private_constructor_markers,
                    );
                    object_binding
                })
                .or_else(|| {
                    self.resolve_returned_object_binding_alias_fallback_with_environment(
                        function_name,
                        Some(&mut execution.environment),
                    )
                    .map(|mut object_binding| {
                        self.merge_static_local_private_constructor_markers(
                            &mut object_binding,
                            &return_value,
                            &mut execution.environment,
                            &local_private_constructor_markers,
                        );
                        object_binding
                    })
                });
        }

        self.resolve_returned_object_binding_alias_fallback_with_environment(
            function_name,
            Some(&mut execution.environment),
        )
        .map(|mut object_binding| {
            let function = self.resolve_registered_function_declaration(function_name)?;
            let returned_expression = match function.body.last()? {
                Statement::Return(expression) => expression.clone(),
                _ => collect_returned_identifier_source_expression(&function.body)?,
            };
            let returned_expression = self.resolve_static_direct_eval_construct_return_expression(
                &returned_expression,
                function_name,
            );
            self.merge_static_local_private_constructor_markers(
                &mut object_binding,
                &returned_expression,
                &mut execution.environment,
                &local_private_constructor_markers,
            );
            Some(object_binding)
        })?
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_returned_descriptor_binding_from_user_function_call(
        &self,
        function_name: &str,
        arguments: &[CallArgument],
    ) -> Option<PropertyDescriptorBinding> {
        let user_function = self.user_function(function_name)?;
        let mut execution = self.prepare_static_user_function_execution(
            function_name,
            user_function,
            arguments,
            &Expression::Undefined,
            None,
            HashMap::new(),
            |statement| statement,
        )?;
        let return_value = self.execute_static_statements_with_state(
            &execution.substituted_body,
            &mut execution.environment,
        )??;
        self.resolve_descriptor_binding_from_expression_with_state(
            &return_value,
            &execution.environment,
        )
    }
}
