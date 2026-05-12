use super::*;

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

    pub(in crate::backend::direct_wasm) fn infer_static_class_init_call_result_expression(
        &self,
        function_name: &str,
    ) -> Option<Expression> {
        if !function_name.starts_with("__ayy_class_init_") {
            return None;
        }
        let function = self.registered_function(function_name)?;
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
