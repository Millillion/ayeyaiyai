use super::*;

fn class_init_descriptor_data_value(expression: &Expression) -> Option<&Expression> {
    let Expression::Object(entries) = expression else {
        return None;
    };

    entries.iter().find_map(|entry| match entry {
        ObjectEntry::Data { key, value }
            if matches!(key, Expression::String(name) if name == "value") =>
        {
            Some(value)
        }
        _ => None,
    })
}

fn class_init_define_property_data_value(expression: &Expression) -> Option<&Expression> {
    let Expression::Call { callee, arguments } = expression else {
        return None;
    };
    let Expression::Member { object, property } = callee.as_ref() else {
        return None;
    };
    if !matches!(
        object.as_ref(),
        Expression::Identifier(name) if name == "Object" || name == "Reflect"
    ) || !matches!(property.as_ref(), Expression::String(name) if name == "defineProperty")
    {
        return None;
    }
    let Some(CallArgument::Expression(descriptor)) = arguments.get(2) else {
        return None;
    };
    class_init_descriptor_data_value(descriptor)
}

fn class_init_expression_has_external_side_effects(expression: &Expression) -> bool {
    if let Some(value) = class_init_define_property_data_value(expression) {
        if !inline_summary_side_effect_free_expression(value) {
            return true;
        }
    }

    match expression {
        Expression::Array(elements) => elements.iter().any(|element| match element {
            ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                class_init_expression_has_external_side_effects(expression)
            }
        }),
        Expression::Object(entries) => entries.iter().any(|entry| match entry {
            ObjectEntry::Data { key, value } => {
                class_init_expression_has_external_side_effects(key)
                    || class_init_expression_has_external_side_effects(value)
            }
            ObjectEntry::Getter { key, getter } => {
                class_init_expression_has_external_side_effects(key)
                    || class_init_expression_has_external_side_effects(getter)
            }
            ObjectEntry::Setter { key, setter } => {
                class_init_expression_has_external_side_effects(key)
                    || class_init_expression_has_external_side_effects(setter)
            }
            ObjectEntry::Spread(expression) => {
                class_init_expression_has_external_side_effects(expression)
            }
        }),
        Expression::Member { object, property } => {
            class_init_expression_has_external_side_effects(object)
                || class_init_expression_has_external_side_effects(property)
        }
        Expression::SuperMember { property } => {
            class_init_expression_has_external_side_effects(property)
        }
        Expression::Assign { value, .. }
        | Expression::AssignMember { value, .. }
        | Expression::AssignSuperMember { value, .. }
        | Expression::Await(value)
        | Expression::EnumerateKeys(value)
        | Expression::GetIterator(value)
        | Expression::IteratorClose(value)
        | Expression::Unary {
            expression: value, ..
        } => class_init_expression_has_external_side_effects(value),
        Expression::Binary { left, right, .. } => {
            class_init_expression_has_external_side_effects(left)
                || class_init_expression_has_external_side_effects(right)
        }
        Expression::Conditional {
            condition,
            then_expression,
            else_expression,
        } => {
            class_init_expression_has_external_side_effects(condition)
                || class_init_expression_has_external_side_effects(then_expression)
                || class_init_expression_has_external_side_effects(else_expression)
        }
        Expression::Sequence(expressions) => expressions
            .iter()
            .any(class_init_expression_has_external_side_effects),
        Expression::Call { callee, arguments }
        | Expression::SuperCall { callee, arguments }
        | Expression::New { callee, arguments } => {
            class_init_expression_has_external_side_effects(callee)
                || arguments.iter().any(|argument| {
                    class_init_expression_has_external_side_effects(argument.expression())
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

fn class_init_statement_has_external_side_effects(statement: &Statement) -> bool {
    match statement {
        Statement::Declaration { body }
        | Statement::Block { body }
        | Statement::Labeled { body, .. }
        | Statement::With { body, .. } => body
            .iter()
            .any(class_init_statement_has_external_side_effects),
        Statement::Var { value, .. }
        | Statement::Let { value, .. }
        | Statement::Assign { value, .. }
        | Statement::Throw(value)
        | Statement::Return(value)
        | Statement::Yield { value }
        | Statement::YieldDelegate { value }
        | Statement::Expression(value) => class_init_expression_has_external_side_effects(value),
        Statement::AssignMember {
            object,
            property,
            value,
        } => {
            class_init_expression_has_external_side_effects(object)
                || class_init_expression_has_external_side_effects(property)
                || class_init_expression_has_external_side_effects(value)
        }
        Statement::Print { values } => values
            .iter()
            .any(class_init_expression_has_external_side_effects),
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            class_init_expression_has_external_side_effects(condition)
                || then_branch
                    .iter()
                    .any(class_init_statement_has_external_side_effects)
                || else_branch
                    .iter()
                    .any(class_init_statement_has_external_side_effects)
        }
        Statement::Try {
            body,
            catch_setup,
            catch_body,
            ..
        } => body
            .iter()
            .chain(catch_setup)
            .chain(catch_body)
            .any(class_init_statement_has_external_side_effects),
        Statement::Switch {
            discriminant,
            cases,
            ..
        } => {
            class_init_expression_has_external_side_effects(discriminant)
                || cases.iter().any(|case| {
                    case.test
                        .as_ref()
                        .is_some_and(class_init_expression_has_external_side_effects)
                        || case
                            .body
                            .iter()
                            .any(class_init_statement_has_external_side_effects)
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
            init.iter()
                .any(class_init_statement_has_external_side_effects)
                || condition
                    .as_ref()
                    .is_some_and(class_init_expression_has_external_side_effects)
                || update
                    .as_ref()
                    .is_some_and(class_init_expression_has_external_side_effects)
                || break_hook
                    .as_ref()
                    .is_some_and(class_init_expression_has_external_side_effects)
                || body
                    .iter()
                    .any(class_init_statement_has_external_side_effects)
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
            class_init_expression_has_external_side_effects(condition)
                || break_hook
                    .as_ref()
                    .is_some_and(class_init_expression_has_external_side_effects)
                || body
                    .iter()
                    .any(class_init_statement_has_external_side_effects)
        }
        Statement::Break { .. } | Statement::Continue { .. } => false,
    }
}

impl<'a> FunctionCompiler<'a> {
    fn registered_static_class_init_function_declaration(
        &self,
        function_name: &str,
    ) -> Option<&FunctionDeclaration> {
        if !function_name.starts_with("__ayy_class_init_") {
            return None;
        }
        if let Some(function) = self.resolve_registered_function_declaration(function_name) {
            return Some(function);
        }

        let namespace_suffix = function_name
            .find("____evalctx_")
            .map(|index| &function_name[index..])?;
        let mut matches = self
            .backend
            .function_registry
            .catalog
            .registered_function_declarations
            .iter()
            .filter(|function| {
                function.name.starts_with("__ayy_class_init_")
                    && function.name.ends_with(namespace_suffix)
            });
        let first = matches.next()?;
        matches.next().is_none().then_some(first)
    }

    fn resolve_static_class_init_local_identifier(
        &self,
        name: &str,
        local_bindings: &std::collections::HashMap<String, Expression>,
    ) -> Expression {
        let mut current = Expression::Identifier(name.to_string());
        let mut seen = std::collections::HashSet::new();
        while let Expression::Identifier(current_name) = &current {
            if !seen.insert(current_name.clone()) {
                break;
            }
            let Some(next) = local_bindings.get(current_name) else {
                break;
            };
            current = next.clone();
        }
        current
    }

    fn resolve_static_class_init_local_expression(
        &self,
        expression: &Expression,
        local_bindings: &std::collections::HashMap<String, Expression>,
    ) -> Expression {
        match expression {
            Expression::Identifier(name) => {
                self.resolve_static_class_init_local_identifier(name, local_bindings)
            }
            Expression::Member { object, property } => Expression::Member {
                object: Box::new(
                    self.resolve_static_class_init_local_expression(object, local_bindings),
                ),
                property: Box::new(
                    self.resolve_static_class_init_local_expression(property, local_bindings),
                ),
            },
            Expression::Call { callee, arguments } => Expression::Call {
                callee: Box::new(
                    self.resolve_static_class_init_local_expression(callee, local_bindings),
                ),
                arguments: arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(expression) => CallArgument::Expression(
                            self.resolve_static_class_init_local_expression(
                                expression,
                                local_bindings,
                            ),
                        ),
                        CallArgument::Spread(expression) => {
                            CallArgument::Spread(self.resolve_static_class_init_local_expression(
                                expression,
                                local_bindings,
                            ))
                        }
                    })
                    .collect(),
            },
            Expression::New { callee, arguments } => Expression::New {
                callee: Box::new(
                    self.resolve_static_class_init_local_expression(callee, local_bindings),
                ),
                arguments: arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(expression) => CallArgument::Expression(
                            self.resolve_static_class_init_local_expression(
                                expression,
                                local_bindings,
                            ),
                        ),
                        CallArgument::Spread(expression) => {
                            CallArgument::Spread(self.resolve_static_class_init_local_expression(
                                expression,
                                local_bindings,
                            ))
                        }
                    })
                    .collect(),
            },
            Expression::Assign { name, value } => Expression::Assign {
                name: name.clone(),
                value: Box::new(
                    self.resolve_static_class_init_local_expression(value, local_bindings),
                ),
            },
            Expression::AssignMember {
                object,
                property,
                value,
            } => Expression::AssignMember {
                object: Box::new(
                    self.resolve_static_class_init_local_expression(object, local_bindings),
                ),
                property: Box::new(
                    self.resolve_static_class_init_local_expression(property, local_bindings),
                ),
                value: Box::new(
                    self.resolve_static_class_init_local_expression(value, local_bindings),
                ),
            },
            Expression::Unary { op, expression } => Expression::Unary {
                op: *op,
                expression: Box::new(
                    self.resolve_static_class_init_local_expression(expression, local_bindings),
                ),
            },
            Expression::Binary { op, left, right } => Expression::Binary {
                op: *op,
                left: Box::new(
                    self.resolve_static_class_init_local_expression(left, local_bindings),
                ),
                right: Box::new(
                    self.resolve_static_class_init_local_expression(right, local_bindings),
                ),
            },
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Expression::Conditional {
                    condition: Box::new(
                        self.resolve_static_class_init_local_expression(condition, local_bindings),
                    ),
                    then_expression: Box::new(self.resolve_static_class_init_local_expression(
                        then_expression,
                        local_bindings,
                    )),
                    else_expression: Box::new(self.resolve_static_class_init_local_expression(
                        else_expression,
                        local_bindings,
                    )),
                }
            }
            Expression::Sequence(expressions) => Expression::Sequence(
                expressions
                    .iter()
                    .map(|expression| {
                        self.resolve_static_class_init_local_expression(expression, local_bindings)
                    })
                    .collect(),
            ),
            Expression::Array(elements) => Expression::Array(
                elements
                    .iter()
                    .map(|element| match element {
                        ArrayElement::Expression(expression) => ArrayElement::Expression(
                            self.resolve_static_class_init_local_expression(
                                expression,
                                local_bindings,
                            ),
                        ),
                        ArrayElement::Spread(expression) => {
                            ArrayElement::Spread(self.resolve_static_class_init_local_expression(
                                expression,
                                local_bindings,
                            ))
                        }
                    })
                    .collect(),
            ),
            Expression::Object(entries) => Expression::Object(
                entries
                    .iter()
                    .map(|entry| match entry {
                        ObjectEntry::Data { key, value } => ObjectEntry::Data {
                            key: self
                                .resolve_static_class_init_local_expression(key, local_bindings),
                            value: self
                                .resolve_static_class_init_local_expression(value, local_bindings),
                        },
                        ObjectEntry::Getter { key, getter } => ObjectEntry::Getter {
                            key: self
                                .resolve_static_class_init_local_expression(key, local_bindings),
                            getter: self
                                .resolve_static_class_init_local_expression(getter, local_bindings),
                        },
                        ObjectEntry::Setter { key, setter } => ObjectEntry::Setter {
                            key: self
                                .resolve_static_class_init_local_expression(key, local_bindings),
                            setter: self
                                .resolve_static_class_init_local_expression(setter, local_bindings),
                        },
                        ObjectEntry::Spread(expression) => {
                            ObjectEntry::Spread(self.resolve_static_class_init_local_expression(
                                expression,
                                local_bindings,
                            ))
                        }
                    })
                    .collect(),
            ),
            _ => expression.clone(),
        }
    }

    pub(in crate::backend::direct_wasm) fn infer_static_class_init_call_result_expression(
        &self,
        function_name: &str,
    ) -> Option<Expression> {
        let function = self.registered_static_class_init_function_declaration(function_name)?;
        if function
            .body
            .iter()
            .any(class_init_statement_has_external_side_effects)
        {
            return None;
        }
        let mut local_bindings = std::collections::HashMap::new();

        for statement in &function.body {
            match statement {
                Statement::Var { name, value } | Statement::Let { name, value, .. } => {
                    local_bindings.insert(
                        name.clone(),
                        self.resolve_static_class_init_local_expression(value, &local_bindings),
                    );
                }
                Statement::Assign { name, value } => {
                    local_bindings.insert(
                        name.clone(),
                        self.resolve_static_class_init_local_expression(value, &local_bindings),
                    );
                }
                Statement::Return(value) => {
                    return Some(
                        self.resolve_static_class_init_local_expression(value, &local_bindings),
                    );
                }
                _ => {}
            }
        }

        None
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_class_init_constructor_alias(
        &self,
        class_binding_name: &str,
    ) -> Option<String> {
        self.backend
            .function_registry
            .catalog
            .registered_function_declarations
            .iter()
            .find(|function| {
                function.name.starts_with("__ayy_class_init_")
                    && function.body.iter().any(|statement| {
                        matches!(
                            statement,
                            Statement::Return(Expression::Identifier(name))
                                if name == class_binding_name
                        )
                    })
            })
            .and_then(|function| {
                self.infer_static_class_init_call_result_expression(&function.name)
            })
            .and_then(|result| match result {
                Expression::Identifier(name) => Some(name),
                _ => None,
            })
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_class_init_call_constructor_alias(
        &self,
        function_name: &str,
    ) -> Option<String> {
        let function = self.registered_static_class_init_function_declaration(function_name)?;
        let mut aliases = std::collections::HashMap::new();

        for statement in &function.body {
            match statement {
                Statement::Var { name, value }
                | Statement::Let { name, value, .. }
                | Statement::Assign { name, value } => {
                    aliases.insert(name.clone(), value.clone());
                }
                Statement::Return(Expression::Identifier(name)) => {
                    let mut current = name.clone();
                    let mut visited = std::collections::HashSet::new();
                    while visited.insert(current.clone()) {
                        let Some(Expression::Identifier(next)) = aliases.get(&current) else {
                            break;
                        };
                        current = next.clone();
                    }
                    return self
                        .resolve_registered_function_declaration(&current)
                        .map(|_| current);
                }
                _ => {}
            }
        }

        None
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_class_init_class_binding_for_constructor_alias(
        &self,
        constructor_name: &str,
    ) -> Option<String> {
        for function in &self
            .backend
            .function_registry
            .catalog
            .registered_function_declarations
        {
            if !function.name.starts_with("__ayy_class_init_") {
                continue;
            }

            let mut aliases = std::collections::HashMap::new();
            for statement in &function.body {
                match statement {
                    Statement::Var { name, value }
                    | Statement::Let { name, value, .. }
                    | Statement::Assign { name, value } => {
                        aliases.insert(name.clone(), value.clone());
                    }
                    Statement::Return(Expression::Identifier(name)) => {
                        let mut current = name.clone();
                        let mut visited = std::collections::HashSet::new();
                        while visited.insert(current.clone()) {
                            let Some(Expression::Identifier(next)) = aliases.get(&current) else {
                                break;
                            };
                            current = next.clone();
                        }
                        if current == constructor_name {
                            return Some(name.clone());
                        }
                    }
                    _ => {}
                }
            }
        }

        None
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_class_init_prototype_parent_expression(
        &self,
        class_or_constructor_name: &str,
    ) -> Option<Expression> {
        for function in &self
            .backend
            .function_registry
            .catalog
            .registered_function_declarations
        {
            if !function.name.starts_with("__ayy_class_init_") {
                continue;
            }

            let mut local_bindings = std::collections::HashMap::new();
            for statement in &function.body {
                match statement {
                    Statement::Var { name, value }
                    | Statement::Let { name, value, .. }
                    | Statement::Assign { name, value } => {
                        local_bindings.insert(
                            name.clone(),
                            self.resolve_static_class_init_local_expression(value, &local_bindings),
                        );
                    }
                    Statement::Expression(Expression::Call { callee, arguments }) if matches!(callee.as_ref(), Expression::Identifier(name) if name == "__ayyClassPrototypeInit") =>
                    {
                        let [
                            CallArgument::Expression(target),
                            CallArgument::Expression(prototype_parent),
                            ..,
                        ] = arguments.as_slice()
                        else {
                            continue;
                        };
                        let raw_target_matches = matches!(
                            target,
                            Expression::Identifier(name) if name == class_or_constructor_name
                        );
                        let resolved_target = self
                            .resolve_static_class_init_local_expression(target, &local_bindings);
                        let resolved_target_matches = matches!(
                            &resolved_target,
                            Expression::Identifier(name) if name == class_or_constructor_name
                        );
                        if !raw_target_matches && !resolved_target_matches {
                            continue;
                        }
                        let prototype_parent = self.resolve_static_class_init_local_expression(
                            prototype_parent,
                            &local_bindings,
                        );
                        return Some(match prototype_parent {
                            Expression::Sequence(expressions) => {
                                expressions.last().cloned().unwrap_or(Expression::Undefined)
                            }
                            other => other,
                        });
                    }
                    _ => {}
                }
            }
        }

        None
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_class_init_local_alias_expression(
        &self,
        alias_name: &str,
    ) -> Option<Expression> {
        for function in &self
            .backend
            .function_registry
            .catalog
            .registered_function_declarations
        {
            if !function.name.starts_with("__ayy_class_init_") {
                continue;
            }

            let mut local_bindings = std::collections::HashMap::new();
            for statement in &function.body {
                match statement {
                    Statement::Var { name, value } | Statement::Let { name, value, .. } => {
                        let resolved =
                            self.resolve_static_class_init_local_expression(value, &local_bindings);
                        if name == alias_name {
                            return Some(resolved);
                        }
                        local_bindings.insert(name.clone(), resolved);
                    }
                    Statement::Assign { name, value } => {
                        let resolved =
                            self.resolve_static_class_init_local_expression(value, &local_bindings);
                        if name == alias_name {
                            return Some(resolved);
                        }
                        local_bindings.insert(name.clone(), resolved);
                    }
                    _ => {}
                }
            }
        }

        None
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_class_init_local_aliases_in_expression(
        &self,
        expression: &Expression,
    ) -> Expression {
        match expression {
            Expression::Identifier(name) => self
                .resolve_static_class_init_local_alias_expression(name)
                .filter(|resolved| !static_expression_matches(resolved, expression))
                .map(|resolved| {
                    self.resolve_static_class_init_local_aliases_in_expression(&resolved)
                })
                .unwrap_or_else(|| expression.clone()),
            Expression::Member { object, property } => Expression::Member {
                object: Box::new(
                    self.resolve_static_class_init_local_aliases_in_expression(object),
                ),
                property: Box::new(
                    self.resolve_static_class_init_local_aliases_in_expression(property),
                ),
            },
            Expression::Sequence(expressions) => Expression::Sequence(
                expressions
                    .iter()
                    .map(|expression| {
                        self.resolve_static_class_init_local_aliases_in_expression(expression)
                    })
                    .collect(),
            ),
            _ => expression.clone(),
        }
    }

    fn static_class_init_constructor_target_matches(
        &self,
        target: &Expression,
        returned_constructor: &Expression,
    ) -> bool {
        if static_expression_matches(target, returned_constructor) {
            return true;
        }
        let resolved = self.resolve_static_class_init_local_aliases_in_expression(target);
        static_expression_matches(&resolved, returned_constructor)
    }

    fn infer_static_class_init_constructor_define_property(
        &self,
        arguments: &[CallArgument],
        returned_constructor: &Expression,
        local_bindings: &std::collections::HashMap<String, Expression>,
        constructor_binding: &mut ObjectValueBinding,
    ) -> bool {
        let [
            CallArgument::Expression(target_expression),
            CallArgument::Expression(property_expression),
            CallArgument::Expression(descriptor_expression),
            ..,
        ] = arguments
        else {
            return false;
        };
        let resolved_target =
            self.resolve_static_class_init_local_expression(target_expression, local_bindings);
        if !self
            .static_class_init_constructor_target_matches(&resolved_target, returned_constructor)
        {
            return false;
        }
        let Some(descriptor) = resolve_property_descriptor_definition(descriptor_expression) else {
            return false;
        };
        let property =
            self.resolve_static_class_init_local_expression(property_expression, local_bindings);
        let property = self.canonical_object_property_expression(&property);
        let property_name = static_property_name_from_expression(&property);
        let existing_value = object_binding_lookup_value(constructor_binding, &property).cloned();
        let existing_descriptor =
            object_binding_lookup_descriptor(constructor_binding, &property).cloned();
        let current_enumerable = property_name.as_ref().is_some_and(|property_name| {
            !constructor_binding
                .non_enumerable_string_properties
                .iter()
                .any(|hidden_name| hidden_name == property_name)
        });
        let enumerable = descriptor.enumerable.unwrap_or_else(|| {
            existing_descriptor
                .as_ref()
                .map(|descriptor| descriptor.enumerable)
                .unwrap_or(current_enumerable)
        });
        let configurable = descriptor.configurable.unwrap_or_else(|| {
            existing_descriptor
                .as_ref()
                .map(|descriptor| descriptor.configurable)
                .unwrap_or(false)
        });
        let (value, writable, getter, setter, has_get, has_set) = if descriptor.is_accessor() {
            (
                None,
                None,
                descriptor
                    .getter
                    .as_ref()
                    .map(|value| {
                        self.resolve_static_class_init_local_expression(value, local_bindings)
                    })
                    .or_else(|| {
                        existing_descriptor
                            .as_ref()
                            .and_then(|descriptor| descriptor.getter.clone())
                    }),
                descriptor
                    .setter
                    .as_ref()
                    .map(|value| {
                        self.resolve_static_class_init_local_expression(value, local_bindings)
                    })
                    .or_else(|| {
                        existing_descriptor
                            .as_ref()
                            .and_then(|descriptor| descriptor.setter.clone())
                    }),
                descriptor.getter.is_some()
                    || existing_descriptor
                        .as_ref()
                        .is_some_and(|descriptor| descriptor.has_get),
                descriptor.setter.is_some()
                    || existing_descriptor
                        .as_ref()
                        .is_some_and(|descriptor| descriptor.has_set),
            )
        } else {
            let value = descriptor
                .value
                .as_ref()
                .map(|value| self.resolve_static_class_init_local_expression(value, local_bindings))
                .or_else(|| existing_value.clone())
                .or_else(|| {
                    existing_descriptor
                        .as_ref()
                        .and_then(|descriptor| descriptor.value.clone())
                })
                .unwrap_or(Expression::Undefined);
            let writable = descriptor.writable.or_else(|| {
                existing_descriptor
                    .as_ref()
                    .and_then(|descriptor| descriptor.writable)
            });
            (
                Some(value),
                Some(writable.unwrap_or(false)),
                None,
                None,
                false,
                false,
            )
        };
        object_binding_define_property_descriptor(
            constructor_binding,
            property,
            PropertyDescriptorBinding {
                value,
                configurable,
                enumerable,
                writable,
                getter,
                setter,
                has_get,
                has_set,
            },
        );
        true
    }

    fn infer_static_class_init_constructor_assignment(
        &self,
        object: &Expression,
        property: &Expression,
        value: &Expression,
        returned_constructor: &Expression,
        local_bindings: &std::collections::HashMap<String, Expression>,
        constructor_binding: &mut ObjectValueBinding,
    ) -> bool {
        let resolved_object =
            self.resolve_static_class_init_local_expression(object, local_bindings);
        if !self
            .static_class_init_constructor_target_matches(&resolved_object, returned_constructor)
        {
            return false;
        }
        let property = self.resolve_static_class_init_local_expression(property, local_bindings);
        let property = self.canonical_object_property_expression(&property);
        let value = self.resolve_static_class_init_local_expression(value, local_bindings);
        let enumerable = !matches!(
            &property,
            Expression::String(property_name) if property_name.starts_with("__ayy$private$")
        );
        object_binding_define_property(constructor_binding, property, value, enumerable);
        true
    }

    fn infer_static_class_init_constructor_statement(
        &self,
        statement: &Statement,
        returned_constructor: &Expression,
        local_bindings: &mut std::collections::HashMap<String, Expression>,
        constructor_binding: &mut ObjectValueBinding,
        found_property: &mut bool,
    ) {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => {
                for statement in body {
                    self.infer_static_class_init_constructor_statement(
                        statement,
                        returned_constructor,
                        local_bindings,
                        constructor_binding,
                        found_property,
                    );
                }
            }
            Statement::Var { name, value } | Statement::Let { name, value, .. } => {
                local_bindings.insert(
                    name.clone(),
                    self.resolve_static_class_init_local_expression(value, local_bindings),
                );
            }
            Statement::Assign { name, value } => {
                local_bindings.insert(
                    name.clone(),
                    self.resolve_static_class_init_local_expression(value, local_bindings),
                );
            }
            Statement::Expression(Expression::Call { callee, arguments })
                if matches!(
                    callee.as_ref(),
                    Expression::Member { object, property }
                        if matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
                            && matches!(property.as_ref(), Expression::String(name) if name == "defineProperty")
                ) =>
            {
                *found_property |= self.infer_static_class_init_constructor_define_property(
                    arguments,
                    returned_constructor,
                    local_bindings,
                    constructor_binding,
                );
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                *found_property |= self.infer_static_class_init_constructor_assignment(
                    object,
                    property,
                    value,
                    returned_constructor,
                    local_bindings,
                    constructor_binding,
                );
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition =
                    self.resolve_static_class_init_local_expression(condition, local_bindings);
                let Some(condition_value) = self.resolve_static_if_condition_value(&condition)
                else {
                    return;
                };
                let branch = if condition_value {
                    then_branch
                } else {
                    else_branch
                };
                for statement in branch {
                    self.infer_static_class_init_constructor_statement(
                        statement,
                        returned_constructor,
                        local_bindings,
                        constructor_binding,
                        found_property,
                    );
                }
            }
            _ => {}
        }
    }

    pub(in crate::backend::direct_wasm) fn infer_static_class_init_constructor_object_binding(
        &self,
        function_name: &str,
    ) -> Option<ObjectValueBinding> {
        let function = self.registered_static_class_init_function_declaration(function_name)?;
        if function
            .body
            .iter()
            .any(class_init_statement_has_external_side_effects)
        {
            return None;
        }
        let body = function.body.clone();
        let returned_constructor =
            self.infer_static_class_init_call_result_expression(function_name)?;
        let mut local_bindings = std::collections::HashMap::new();
        let mut constructor_binding = empty_object_value_binding();
        let mut found_property = false;

        for statement in &body {
            self.infer_static_class_init_constructor_statement(
                statement,
                &returned_constructor,
                &mut local_bindings,
                &mut constructor_binding,
                &mut found_property,
            );
        }

        found_property.then_some(constructor_binding)
    }

    fn resolve_static_class_init_storage_name_from_expression(
        &self,
        expression: &Expression,
    ) -> Option<String> {
        match expression {
            Expression::Identifier(name) => self
                .resolve_static_class_init_local_alias_expression(name)
                .and_then(|resolved| {
                    self.resolve_static_class_init_storage_name_from_expression(&resolved)
                })
                .or_else(|| {
                    self.infer_static_class_init_prototype_object_binding(name)
                        .is_some()
                        .then(|| name.clone())
                })
                .or_else(|| {
                    self.resolve_function_binding_from_expression(expression)
                        .and_then(|binding| self.function_prototype_binding_owner_name(&binding))
                }),
            Expression::Call { callee, .. } => {
                let Expression::Identifier(function_name) = callee.as_ref() else {
                    return None;
                };
                self.infer_static_class_init_call_result_expression(function_name)
                    .and_then(|result| match result {
                        Expression::Identifier(name) => Some(name),
                        _ => None,
                    })
            }
            _ => None,
        }
    }

    fn infer_static_class_init_prototype_object_binding(
        &self,
        constructor_name: &str,
    ) -> Option<ObjectValueBinding> {
        let init_function = self
            .backend
            .function_registry
            .catalog
            .registered_function_declarations
            .iter()
            .find(|function| {
                let Some(Expression::Identifier(returned_name)) =
                    self.infer_static_class_init_call_result_expression(&function.name)
                else {
                    return false;
                };
                returned_name == constructor_name
                    || self
                        .resolve_registered_function_declaration(&returned_name)
                        .and_then(|returned_function| {
                            returned_function
                                .self_binding
                                .as_ref()
                                .or(returned_function.top_level_binding.as_ref())
                        })
                        .is_some_and(|owner_name| owner_name == constructor_name)
            })?;

        let mut local_bindings = std::collections::HashMap::new();
        let mut prototype_binding = empty_object_value_binding();
        let mut found_property = false;

        for statement in &init_function.body {
            match statement {
                Statement::Var { name, value } | Statement::Let { name, value, .. } => {
                    local_bindings.insert(
                        name.clone(),
                        self.resolve_static_class_init_local_expression(value, &local_bindings),
                    );
                }
                Statement::Assign { name, value } => {
                    local_bindings.insert(
                        name.clone(),
                        self.resolve_static_class_init_local_expression(value, &local_bindings),
                    );
                }
                Statement::Expression(Expression::Call { callee, arguments })
                    if matches!(
                        callee.as_ref(),
                        Expression::Member { object, property }
                            if matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
                                && matches!(property.as_ref(), Expression::String(name) if name == "defineProperty")
                    ) =>
                {
                    let [
                        CallArgument::Expression(target_expression),
                        CallArgument::Expression(property_expression),
                        CallArgument::Expression(descriptor_expression),
                        ..,
                    ] = arguments.as_slice()
                    else {
                        continue;
                    };
                    let resolved_target = self.resolve_static_class_init_local_expression(
                        target_expression,
                        &local_bindings,
                    );
                    let resolved_constructor = self.resolve_static_class_init_local_expression(
                        &Expression::Identifier(constructor_name.to_string()),
                        &local_bindings,
                    );
                    let is_constructor_prototype = matches!(
                        &resolved_target,
                        Expression::Member { object, property }
                            if (matches!(object.as_ref(), Expression::Identifier(name) if name == constructor_name)
                                || static_expression_matches(object.as_ref(), &resolved_constructor))
                                && matches!(property.as_ref(), Expression::String(name) if name == "prototype")
                    );
                    if !is_constructor_prototype {
                        continue;
                    }
                    let Some(descriptor) =
                        resolve_property_descriptor_definition(descriptor_expression)
                    else {
                        continue;
                    };
                    let property = self.resolve_static_class_init_local_expression(
                        property_expression,
                        &local_bindings,
                    );
                    let property = self.canonical_object_property_expression(&property);
                    let property_name = static_property_name_from_expression(&property);
                    let existing_value =
                        object_binding_lookup_value(&prototype_binding, &property).cloned();
                    let existing_descriptor =
                        object_binding_lookup_descriptor(&prototype_binding, &property).cloned();
                    let current_enumerable = property_name.as_ref().is_some_and(|property_name| {
                        !prototype_binding
                            .non_enumerable_string_properties
                            .iter()
                            .any(|hidden_name| hidden_name == property_name)
                    });
                    let enumerable = descriptor.enumerable.unwrap_or_else(|| {
                        existing_descriptor
                            .as_ref()
                            .map(|descriptor| descriptor.enumerable)
                            .unwrap_or(current_enumerable)
                    });
                    let configurable = descriptor.configurable.unwrap_or_else(|| {
                        existing_descriptor
                            .as_ref()
                            .map(|descriptor| descriptor.configurable)
                            .unwrap_or(false)
                    });
                    let (value, writable, getter, setter, has_get, has_set) =
                        if descriptor.is_accessor() {
                            (
                                None,
                                None,
                                descriptor
                                    .getter
                                    .as_ref()
                                    .map(|value| {
                                        self.resolve_static_class_init_local_expression(
                                            value,
                                            &local_bindings,
                                        )
                                    })
                                    .or_else(|| {
                                        existing_descriptor
                                            .as_ref()
                                            .and_then(|descriptor| descriptor.getter.clone())
                                    }),
                                descriptor
                                    .setter
                                    .as_ref()
                                    .map(|value| {
                                        self.resolve_static_class_init_local_expression(
                                            value,
                                            &local_bindings,
                                        )
                                    })
                                    .or_else(|| {
                                        existing_descriptor
                                            .as_ref()
                                            .and_then(|descriptor| descriptor.setter.clone())
                                    }),
                                descriptor.getter.is_some()
                                    || existing_descriptor
                                        .as_ref()
                                        .is_some_and(|descriptor| descriptor.has_get),
                                descriptor.setter.is_some()
                                    || existing_descriptor
                                        .as_ref()
                                        .is_some_and(|descriptor| descriptor.has_set),
                            )
                        } else {
                            let value = descriptor
                                .value
                                .as_ref()
                                .map(|value| {
                                    self.resolve_static_class_init_local_expression(
                                        value,
                                        &local_bindings,
                                    )
                                })
                                .or_else(|| existing_value.clone())
                                .or_else(|| {
                                    existing_descriptor
                                        .as_ref()
                                        .and_then(|descriptor| descriptor.value.clone())
                                })
                                .unwrap_or(Expression::Undefined);
                            let writable = descriptor.writable.or_else(|| {
                                existing_descriptor
                                    .as_ref()
                                    .and_then(|descriptor| descriptor.writable)
                            });
                            (
                                Some(value),
                                Some(writable.unwrap_or(false)),
                                None,
                                None,
                                false,
                                false,
                            )
                        };
                    object_binding_define_property_descriptor(
                        &mut prototype_binding,
                        property,
                        PropertyDescriptorBinding {
                            value,
                            configurable,
                            enumerable,
                            writable,
                            getter,
                            setter,
                            has_get,
                            has_set,
                        },
                    );
                    found_property = true;
                }
                _ => {}
            }
        }

        found_property.then_some(prototype_binding)
    }

    pub(in crate::backend::direct_wasm) fn function_prototype_binding_owner_name(
        &self,
        binding: &LocalFunctionBinding,
    ) -> Option<String> {
        match binding {
            LocalFunctionBinding::User(function_name) => Some(
                self.resolve_registered_function_declaration(function_name)
                    .and_then(|function| {
                        function
                            .self_binding
                            .as_ref()
                            .or(function.top_level_binding.as_ref())
                            .cloned()
                    })
                    .unwrap_or_else(|| function_name.clone()),
            ),
            LocalFunctionBinding::Builtin(function_name) => Some(function_name.clone()),
        }
    }

    pub(in crate::backend::direct_wasm) fn merge_object_binding_properties(
        target: &mut ObjectValueBinding,
        source: &ObjectValueBinding,
    ) {
        for (name, value) in &source.string_properties {
            let enumerable = !source
                .non_enumerable_string_properties
                .iter()
                .any(|hidden_name| hidden_name == name);
            object_binding_define_property(
                target,
                Expression::String(name.clone()),
                value.clone(),
                enumerable,
            );
        }
        for (property, value) in &source.symbol_properties {
            object_binding_define_property(target, property.clone(), value.clone(), true);
        }
        for (property, descriptor) in &source.property_descriptors {
            object_binding_define_property_descriptor(target, property.clone(), descriptor.clone());
        }
    }

    pub(in crate::backend::direct_wasm) fn default_function_prototype_object_binding(
        &self,
        function_binding: &LocalFunctionBinding,
    ) -> Option<ObjectValueBinding> {
        let constructor_expression = match function_binding {
            LocalFunctionBinding::User(function_name) => {
                let user_function = self.user_function(function_name)?;
                if user_function.is_generator() {
                    return Some(empty_object_value_binding());
                }
                if !user_function.is_constructible() {
                    return None;
                }
                Expression::Identifier(function_name.clone())
            }
            LocalFunctionBinding::Builtin(function_name) => {
                if !is_function_constructor_builtin(function_name) {
                    return None;
                }
                Expression::Identifier(function_name.clone())
            }
        };

        let mut object_binding = empty_object_value_binding();
        object_binding_define_property(
            &mut object_binding,
            Expression::String("constructor".to_string()),
            constructor_expression,
            false,
        );
        if matches!(
            function_binding,
            LocalFunctionBinding::Builtin(function_name)
                if matches!(function_name.as_str(), "GeneratorFunction" | "AsyncGeneratorFunction")
        ) {
            object_binding_define_property(
                &mut object_binding,
                Expression::String("prototype".to_string()),
                Expression::Object(Vec::new()),
                false,
            );
        }
        Some(object_binding)
    }

    pub(in crate::backend::direct_wasm) fn resolve_function_prototype_object_binding(
        &self,
        name: &str,
    ) -> Option<ObjectValueBinding> {
        let resolved_storage_name = self
            .resolve_function_binding_from_expression(&Expression::Identifier(name.to_string()))
            .and_then(|binding| self.function_prototype_binding_owner_name(&binding))
            .or_else(|| self.resolve_static_class_init_constructor_alias(name))
            .or_else(|| {
                self.resolve_static_class_init_local_alias_expression(name)
                    .and_then(|resolved| match resolved {
                        Expression::Identifier(resolved_name) => self
                            .resolve_function_binding_from_expression(&Expression::Identifier(
                                resolved_name.clone(),
                            ))
                            .and_then(|binding| {
                                self.function_prototype_binding_owner_name(&binding)
                            })
                            .or(Some(resolved_name)),
                        _ => None,
                    })
            })
            .or_else(|| {
                self.state
                    .speculation
                    .static_semantics
                    .local_value_binding(name)
                    .or_else(|| self.global_value_binding(name))
                    .and_then(|value| {
                        self.resolve_static_class_init_storage_name_from_expression(value)
                    })
            })
            .unwrap_or_else(|| name.to_string());
        let trace_prototype_bindings = std::env::var_os("AYY_TRACE_PROTOTYPE_BINDINGS").is_some();
        if trace_prototype_bindings {
            eprintln!(
                "prototype_binding:resolve name={name} resolved_storage_name={resolved_storage_name} local_value={:?} global_value={:?}",
                self.state
                    .speculation
                    .static_semantics
                    .local_value_binding(name),
                self.global_value_binding(name),
            );
        }
        let mut prototype_binding_names = vec![name.to_string(), resolved_storage_name.clone()];
        if let Some(Expression::Identifier(alias)) = self
            .state
            .speculation
            .static_semantics
            .local_value_binding(name)
            .or_else(|| self.global_value_binding(name))
        {
            prototype_binding_names.push(alias.clone());
        }
        if let Some(Expression::Identifier(alias)) =
            self.resolve_bound_alias_expression(&Expression::Identifier(name.to_string()))
        {
            prototype_binding_names.push(alias);
        }
        if let Some(Expression::Identifier(alias)) =
            self.resolve_static_class_init_local_alias_expression(name)
        {
            prototype_binding_names.push(alias);
        }
        if let Some(function) = self.resolve_registered_function_declaration(name)
            && let Some(self_binding) = function.self_binding.as_ref()
        {
            prototype_binding_names.push(self_binding.clone());
        }
        if let Some(class_binding_name) =
            self.resolve_static_class_init_class_binding_for_constructor_alias(name)
        {
            prototype_binding_names.push(class_binding_name);
        }
        if resolved_storage_name != name {
            if let Some(Expression::Identifier(alias)) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(&resolved_storage_name)
                .or_else(|| self.global_value_binding(&resolved_storage_name))
            {
                prototype_binding_names.push(alias.clone());
            }
            if let Some(Expression::Identifier(alias)) = self.resolve_bound_alias_expression(
                &Expression::Identifier(resolved_storage_name.clone()),
            ) {
                prototype_binding_names.push(alias);
            }
            if let Some(Expression::Identifier(alias)) =
                self.resolve_static_class_init_local_alias_expression(&resolved_storage_name)
            {
                prototype_binding_names.push(alias);
            }
            if let Some(function) =
                self.resolve_registered_function_declaration(&resolved_storage_name)
                && let Some(self_binding) = function.self_binding.as_ref()
            {
                prototype_binding_names.push(self_binding.clone());
            }
            if let Some(class_binding_name) = self
                .resolve_static_class_init_class_binding_for_constructor_alias(
                    &resolved_storage_name,
                )
            {
                prototype_binding_names.push(class_binding_name);
            }
        }
        let mut candidate_index = 0;
        while candidate_index < prototype_binding_names.len() {
            let candidate_name = prototype_binding_names[candidate_index].clone();
            for (alias_name, alias_value) in self
                .backend
                .global_semantics
                .values
                .value_bindings
                .iter()
                .chain(
                    self.backend
                        .shared_global_semantics
                        .values
                        .value_bindings
                        .iter(),
                )
            {
                if matches!(alias_value, Expression::Identifier(target_name) if target_name == &candidate_name)
                    && !prototype_binding_names
                        .iter()
                        .any(|name| name == alias_name)
                {
                    prototype_binding_names.push(alias_name.clone());
                }
            }
            candidate_index += 1;
        }
        prototype_binding_names.dedup();

        let mut stored_binding: Option<ObjectValueBinding> = None;
        for candidate_name in &prototype_binding_names {
            let candidate_bindings = [
                self.state
                    .speculation
                    .static_semantics
                    .objects
                    .local_prototype_object_bindings
                    .get(candidate_name),
                self.backend
                    .global_semantics
                    .values
                    .prototype_object_bindings
                    .get(candidate_name),
                self.backend
                    .shared_global_semantics
                    .values
                    .prototype_object_bindings
                    .get(candidate_name),
            ];
            for candidate_binding in candidate_bindings.into_iter().flatten() {
                if let Some(stored_binding) = stored_binding.as_mut() {
                    Self::merge_object_binding_properties(stored_binding, candidate_binding);
                } else {
                    stored_binding = Some(candidate_binding.clone());
                }
            }
        }
        let inferred_binding = self
            .infer_static_class_init_prototype_object_binding(name)
            .or_else(|| {
                (resolved_storage_name != name).then(|| {
                    self.infer_static_class_init_prototype_object_binding(&resolved_storage_name)
                })?
            });
        if trace_prototype_bindings {
            eprintln!(
                "prototype_binding:bindings name={name} resolved_storage_name={resolved_storage_name} stored_props={:?} inferred_props={:?}",
                stored_binding
                    .as_ref()
                    .map(ordered_object_property_names)
                    .unwrap_or_default(),
                inferred_binding
                    .as_ref()
                    .map(ordered_object_property_names)
                    .unwrap_or_default(),
            );
        }
        let default_binding = self
            .resolve_function_binding_from_expression(&Expression::Identifier(name.to_string()))
            .or_else(|| {
                (resolved_storage_name != name).then(|| {
                    self.resolve_function_binding_from_expression(&Expression::Identifier(
                        resolved_storage_name.clone(),
                    ))
                })?
            })
            .and_then(|binding| self.default_function_prototype_object_binding(&binding));

        match (default_binding, stored_binding, inferred_binding) {
            (Some(mut default_binding), Some(stored_binding), Some(inferred_binding)) => {
                Self::merge_object_binding_properties(&mut default_binding, &stored_binding);
                Self::merge_object_binding_properties(&mut default_binding, &inferred_binding);
                Some(default_binding)
            }
            (Some(mut default_binding), Some(stored_binding), None) => {
                Self::merge_object_binding_properties(&mut default_binding, &stored_binding);
                Some(default_binding)
            }
            (Some(mut default_binding), None, Some(inferred_binding)) => {
                Self::merge_object_binding_properties(&mut default_binding, &inferred_binding);
                Some(default_binding)
            }
            (None, Some(mut stored_binding), Some(inferred_binding)) => {
                Self::merge_object_binding_properties(&mut stored_binding, &inferred_binding);
                Some(stored_binding)
            }
            (Some(default_binding), None, None) => Some(default_binding),
            (None, Some(stored_binding), None) => Some(stored_binding),
            (None, None, Some(inferred_binding)) => Some(inferred_binding),
            (None, None, None) => None,
        }
    }
}
