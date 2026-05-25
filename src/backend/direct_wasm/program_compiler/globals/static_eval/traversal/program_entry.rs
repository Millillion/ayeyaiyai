use super::*;

impl DirectWasmCompiler {
    fn seed_static_eval_source_bindings(&mut self, statements: &[Statement]) {
        let mut next_realm_id = 0;
        let mut realm_bindings = HashMap::new();
        self.seed_static_eval_source_bindings_with_context(
            statements,
            &mut next_realm_id,
            &mut realm_bindings,
        );
    }

    fn seed_static_eval_source_bindings_with_context(
        &mut self,
        statements: &[Statement],
        next_realm_id: &mut u32,
        realm_bindings: &mut HashMap<String, u32>,
    ) {
        for statement in statements {
            let (name, value) = match statement {
                Statement::Var { name, value }
                | Statement::Let { name, value, .. }
                | Statement::Assign { name, value } => (name, value),
                Statement::Declaration { body }
                | Statement::Block { body }
                | Statement::Labeled { body, .. } => {
                    self.seed_static_eval_source_bindings_with_context(
                        body,
                        next_realm_id,
                        realm_bindings,
                    );
                    continue;
                }
                _ => continue,
            };
            let seeded_value = match value {
                Expression::String(_) | Expression::Identifier(_) => Some(value.clone()),
                Expression::Call { callee, arguments }
                    if arguments.is_empty()
                        && matches!(
                            callee.as_ref(),
                            Expression::Member { object, property }
                                if matches!(object.as_ref(), Expression::Identifier(name) if name == "$262")
                                    && matches!(property.as_ref(), Expression::String(name) if name == "createRealm")
                        ) =>
                {
                    let realm_id = *next_realm_id;
                    *next_realm_id += 1;
                    realm_bindings.insert(name.clone(), realm_id);
                    Some(Expression::Identifier(test262_realm_identifier(realm_id)))
                }
                Expression::Member { object, property } if matches!(property.as_ref(), Expression::String(property_name) if property_name == "global") => {
                    match object.as_ref() {
                        Expression::Call { callee, arguments }
                            if arguments.is_empty()
                                && matches!(
                                    callee.as_ref(),
                                    Expression::Member { object, property }
                                        if matches!(object.as_ref(), Expression::Identifier(name) if name == "$262")
                                            && matches!(property.as_ref(), Expression::String(name) if name == "createRealm")
                                ) =>
                        {
                            let realm_id = *next_realm_id;
                            *next_realm_id += 1;
                            Some(Expression::Identifier(test262_realm_global_identifier(
                                realm_id,
                            )))
                        }
                        Expression::Identifier(realm_name) => {
                            realm_bindings.get(realm_name).map(|realm_id| {
                                Expression::Identifier(test262_realm_global_identifier(*realm_id))
                            })
                        }
                        _ => None,
                    }
                }
                Expression::Member { object, property } if matches!(property.as_ref(), Expression::String(property_name) if property_name == "eval") => {
                    match object.as_ref() {
                        Expression::Member { object, property } if matches!(property.as_ref(), Expression::String(property_name) if property_name == "global") => {
                            match object.as_ref() {
                                Expression::Identifier(realm_name) => {
                                    realm_bindings.get(realm_name).map(|realm_id| {
                                        Expression::Identifier(test262_realm_eval_builtin_name(
                                            *realm_id,
                                        ))
                                    })
                                }
                                _ => None,
                            }
                        }
                        Expression::Identifier(global_name) => {
                            parse_test262_realm_global_identifier(global_name)
                                .or_else(|| {
                                    self.state
                                        .global_semantics
                                        .values
                                        .value_binding(global_name)
                                        .and_then(|value| match value {
                                            Expression::Identifier(realm_global_name) => {
                                                parse_test262_realm_global_identifier(
                                                    realm_global_name,
                                                )
                                            }
                                            _ => None,
                                        })
                                })
                                .map(|realm_id| {
                                    Expression::Identifier(test262_realm_eval_builtin_name(
                                        realm_id,
                                    ))
                                })
                        }
                        _ => None,
                    }
                }
                _ => None,
            };
            if let Some(seeded_value) = seeded_value {
                self.state
                    .global_semantics
                    .values
                    .set_value_binding(name.clone(), seeded_value);
            }
        }
    }

    pub(in crate::backend::direct_wasm) fn register_static_eval_functions(
        &mut self,
        program: &Program,
    ) -> DirectResult<()> {
        self.seed_static_eval_source_bindings(&program.statements);
        self.register_static_eval_functions_in_statements(&program.statements, None)?;
        for function in &program.functions {
            self.register_static_eval_functions_in_statements(
                &function.body,
                Some(function.name.as_str()),
            )?;
        }
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn register_static_eval_functions_in_statements(
        &mut self,
        statements: &[Statement],
        current_function_name: Option<&str>,
    ) -> DirectResult<()> {
        for statement in statements {
            self.register_static_eval_functions_in_statement(statement, current_function_name)?;
        }
        Ok(())
    }
}
