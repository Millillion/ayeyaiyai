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

fn class_init_define_property_property_key(expression: &Expression) -> Option<&Expression> {
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
    let Some(CallArgument::Expression(property)) = arguments.get(1) else {
        return None;
    };
    Some(property)
}

fn class_init_property_key_can_have_to_property_key_side_effects(expression: &Expression) -> bool {
    match expression {
        Expression::String(_)
        | Expression::Number(_)
        | Expression::BigInt(_)
        | Expression::Bool(_)
        | Expression::Null
        | Expression::Undefined => false,
        Expression::Sequence(expressions) => expressions
            .last()
            .is_some_and(class_init_property_key_can_have_to_property_key_side_effects),
        _ => true,
    }
}

fn class_init_expression_has_external_side_effects(expression: &Expression) -> bool {
    if let Some(value) = class_init_define_property_data_value(expression) {
        if !inline_summary_side_effect_free_expression(value) {
            return true;
        }
    }
    if let Some(property_key) = class_init_define_property_property_key(expression) {
        if class_init_property_key_can_have_to_property_key_side_effects(property_key) {
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

impl DirectWasmCompiler {
    fn resolve_static_class_init_local_identifier(
        &self,
        name: &str,
        local_bindings: &HashMap<String, Expression>,
    ) -> Expression {
        let mut current = Expression::Identifier(name.to_string());
        let mut seen = HashSet::new();
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

    pub(in crate::backend::direct_wasm) fn resolve_static_class_init_local_expression(
        &self,
        expression: &Expression,
        local_bindings: &HashMap<String, Expression>,
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

    pub(in crate::backend::direct_wasm) fn infer_static_class_init_function_result_expression(
        &self,
        function: &FunctionDeclaration,
    ) -> Option<Expression> {
        if !function.name.starts_with("__ayy_class_init_") {
            return None;
        }
        if function
            .body
            .iter()
            .any(class_init_statement_has_external_side_effects)
        {
            return None;
        }
        let mut local_bindings = HashMap::new();

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

    pub(in crate::backend::direct_wasm) fn infer_static_class_init_call_result_expression(
        &self,
        function_name: &str,
    ) -> Option<Expression> {
        let function = self.registered_function(function_name)?;
        self.infer_static_class_init_function_result_expression(function)
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_class_init_local_alias_expression(
        &self,
        alias_name: &str,
    ) -> Option<Expression> {
        for function in &self
            .state
            .function_registry
            .catalog
            .registered_function_declarations
        {
            if !function.name.starts_with("__ayy_class_init_") {
                continue;
            }

            let mut local_bindings = HashMap::new();
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

    pub(in crate::backend::direct_wasm) fn infer_static_call_result_expression(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Option<Expression> {
        let Expression::Identifier(_) = callee else {
            return None;
        };
        let binding = self.infer_global_function_binding(callee)?;
        let user_function = match &binding {
            LocalFunctionBinding::User(function_name) => self.user_function(function_name)?,
            LocalFunctionBinding::Builtin(_) => return None,
        };
        if user_function.is_async() || user_function.is_generator() {
            return None;
        }
        if let LocalFunctionBinding::User(function_name) = &binding
            && let Some(result) = self.infer_static_class_init_call_result_expression(function_name)
        {
            return Some(result);
        }

        let context = self.static_eval_context();
        execute_static_user_function_binding_in_global_maps(
            &context,
            &binding,
            arguments,
            &mut HashMap::new(),
            &mut HashMap::new(),
            StaticFunctionEffectMode::Commit,
        )
        .or_else(|| {
            let LocalFunctionBinding::User(function_name) = &binding else {
                return None;
            };
            self.infer_static_class_init_call_result_expression(function_name)
        })
    }
}
