use super::*;

impl ModuleLinker {
    pub(super) fn lower_default_export_declaration(
        &mut self,
        export_default: &ExportDefaultDecl,
        hoisted_statements: &mut Vec<Statement>,
        body_statements: &mut Vec<Statement>,
    ) -> Result<Expression> {
        match &export_default.decl {
            DefaultDecl::Fn(function_expression) => {
                if let Some(identifier) = &function_expression.ident {
                    let generated_name = self
                        .lowerer
                        .lower_named_default_function_expression(function_expression)?;
                    hoisted_statements.push(Statement::Let {
                        name: identifier.sym.to_string(),
                        mutable: true,
                        value: Expression::Identifier(generated_name),
                    });
                    Ok(Expression::Identifier(identifier.sym.to_string()))
                } else {
                    let local_name = self.lowerer.fresh_temporary_name("module_default");
                    hoisted_statements.push(Statement::Let {
                        name: local_name.clone(),
                        mutable: true,
                        value: self
                            .lowerer
                            .lower_function_expression(function_expression, Some("default"))?,
                    });
                    Ok(Expression::Identifier(local_name))
                }
            }
            DefaultDecl::Class(class_expression) => {
                let local_name = class_expression
                    .ident
                    .as_ref()
                    .map(|identifier| identifier.sym.to_string())
                    .unwrap_or_else(|| "default".to_string());
                body_statements.extend(self.lowerer.lower_class_definition_with_mode(
                    &class_expression.class,
                    local_name.clone(),
                    false,
                )?);
                Ok(Expression::Identifier(local_name))
            }
            other => bail!("unsupported default export declaration: {other:?}"),
        }
    }

    pub(super) fn build_module_namespace_prelude_with_tag(
        &self,
        exports_param: &str,
        to_string_tag: &str,
    ) -> Vec<Statement> {
        vec![
            define_property_statement(
                Expression::Identifier(exports_param.to_string()),
                Expression::Member {
                    object: Box::new(Expression::Identifier("Symbol".to_string())),
                    property: Box::new(Expression::String("toStringTag".to_string())),
                },
                Expression::Object(vec![
                    ObjectEntry::Data {
                        key: Expression::String("value".to_string()),
                        value: Expression::String(to_string_tag.to_string()),
                    },
                    ObjectEntry::Data {
                        key: Expression::String("writable".to_string()),
                        value: Expression::Bool(false),
                    },
                    ObjectEntry::Data {
                        key: Expression::String("enumerable".to_string()),
                        value: Expression::Bool(false),
                    },
                    ObjectEntry::Data {
                        key: Expression::String("configurable".to_string()),
                        value: Expression::Bool(false),
                    },
                ]),
            ),
            define_property_statement(
                Expression::Identifier(exports_param.to_string()),
                Expression::String("__ayy$module$namespace".to_string()),
                data_property_descriptor(Expression::Bool(true), false, false, false),
            ),
        ]
    }

    pub(super) fn build_module_namespace_prelude(&self, exports_param: &str) -> Vec<Statement> {
        self.build_module_namespace_prelude_with_tag(exports_param, "Module")
    }

    pub(super) fn module_status_assignment(&self, module_index: usize, status: f64) -> Statement {
        Statement::Assign {
            name: self.modules[module_index].status_name.clone(),
            value: Expression::Number(status),
        }
    }

    pub(super) fn mark_module_init_body_status(
        &self,
        module_index: usize,
        body: &mut Vec<Statement>,
    ) {
        body.insert(0, self.module_status_assignment(module_index, 1.0));
        body.push(self.module_status_assignment(module_index, 2.0));
    }

    pub(super) fn build_export_getter_statements(
        &mut self,
        module_index: usize,
        exports_param: &str,
        export_expressions: &BTreeMap<String, Expression>,
        namespace_export_module_indices: &BTreeMap<String, usize>,
        reexport_sources: &BTreeMap<String, (usize, String)>,
        import_bindings: &HashMap<String, ImportBinding>,
    ) -> Result<Vec<Statement>> {
        let mut statements = Vec::new();

        for (export_name, expression) in export_expressions {
            let getter_name = format!(
                "__ayy_module_export_getter_{}_{}",
                module_index,
                self.lowerer.fresh_temporary_name("getter")
            );
            let mut getter_function = FunctionDeclaration {
                name: getter_name.clone(),
                top_level_binding: None,
                params: Vec::new(),
                body: vec![Statement::Return(expression.clone())],
                register_global: false,
                kind: FunctionKind::Ordinary,
                self_binding: None,
                mapped_arguments: false,
                strict: true,
                lexical_this: false,
                constructible: false,
                derived_constructor: false,
                direct_eval_in_class_field_initializer: false,
                length: 0,
                synthetic_capture_bindings: Vec::new(),
                immutable_class_bindings: Vec::new(),
                private_brand_binding: None,
            };
            rewrite_module_import_bindings_in_function(
                &mut getter_function,
                import_bindings,
                module_index,
            )?;
            self.lowerer.functions.push(getter_function);

            let mut descriptor_entries = vec![
                ObjectEntry::Data {
                    key: Expression::String("get".to_string()),
                    value: Expression::Identifier(getter_name),
                },
                ObjectEntry::Data {
                    key: Expression::String("enumerable".to_string()),
                    value: Expression::Bool(true),
                },
                ObjectEntry::Data {
                    key: Expression::String("configurable".to_string()),
                    value: Expression::Bool(false),
                },
            ];
            if let Some(namespace_module_index) = namespace_export_module_indices.get(export_name) {
                descriptor_entries.push(ObjectEntry::Data {
                    key: Expression::String("__ayy$module$namespace$moduleIndex".to_string()),
                    value: Expression::Number(*namespace_module_index as f64),
                });
            }
            if let Some((module_index, imported_name)) = reexport_sources.get(export_name) {
                descriptor_entries.push(ObjectEntry::Data {
                    key: Expression::String("__ayy$module$reexport$moduleIndex".to_string()),
                    value: Expression::Number(*module_index as f64),
                });
                descriptor_entries.push(ObjectEntry::Data {
                    key: Expression::String("__ayy$module$reexport$name".to_string()),
                    value: Expression::String(imported_name.clone()),
                });
            }

            statements.push(define_property_statement(
                Expression::Identifier(exports_param.to_string()),
                Expression::String(export_name.clone()),
                Expression::Object(descriptor_entries),
            ));
        }

        Ok(statements)
    }

    pub(super) fn module_registry_statements(&self) -> Vec<Statement> {
        let mut statements = Vec::new();

        for (module_index, module) in self.modules.iter().enumerate() {
            statements.push(Statement::Let {
                name: module.namespace_name.clone(),
                mutable: false,
                value: Expression::Call {
                    callee: Box::new(Expression::Member {
                        object: Box::new(Expression::Identifier("Object".to_string())),
                        property: Box::new(Expression::String("create".to_string())),
                    }),
                    arguments: vec![CallArgument::Expression(Expression::Null)],
                },
            });
            statements.push(Statement::Let {
                name: module.deferred_namespace_name.clone(),
                mutable: false,
                value: Expression::Call {
                    callee: Box::new(Expression::Member {
                        object: Box::new(Expression::Identifier("Object".to_string())),
                        property: Box::new(Expression::String("create".to_string())),
                    }),
                    arguments: vec![CallArgument::Expression(Expression::Null)],
                },
            });
            statements.push(Statement::Let {
                name: module.status_name.clone(),
                mutable: true,
                value: Expression::Number(0.0),
            });
            statements.push(Statement::Let {
                name: module.error_name.clone(),
                mutable: true,
                value: Expression::Undefined,
            });
            for dependency in &module.dependency_params {
                if dependency.eager {
                    statements.push(Statement::Let {
                        name: format!(
                            "__ayy_module_eager_dependency_{}_{}",
                            module_index, dependency.module_index
                        ),
                        mutable: false,
                        value: Expression::Bool(true),
                    });
                }
            }
            statements.extend(self.build_module_namespace_prelude_with_tag(
                &module.deferred_namespace_name,
                "Deferred Module",
            ));
            statements.push(Statement::Let {
                name: format!("__ayy_import_meta_{module_index}"),
                mutable: true,
                value: Expression::Call {
                    callee: Box::new(Expression::Member {
                        object: Box::new(Expression::Identifier("Object".to_string())),
                        property: Box::new(Expression::String("create".to_string())),
                    }),
                    arguments: vec![CallArgument::Expression(Expression::Null)],
                },
            });
        }

        statements
    }

    pub(super) fn module_init_call_arguments(&self, module_index: usize) -> Vec<CallArgument> {
        let module = &self.modules[module_index];
        let mut arguments = vec![CallArgument::Expression(Expression::Identifier(
            module.namespace_name.clone(),
        ))];
        for dependency in &module.dependency_params {
            arguments.push(CallArgument::Expression(Expression::Identifier(
                self.modules[dependency.module_index].namespace_name.clone(),
            )));
        }
        arguments
    }

    fn module_eager_dependency_is_pending(
        &self,
        module_index: usize,
        pending: &HashSet<usize>,
        seen: &mut HashSet<usize>,
    ) -> bool {
        if !seen.insert(module_index) {
            return false;
        }

        self.modules[module_index]
            .dependency_params
            .iter()
            .filter(|dependency| dependency.eager)
            .any(|dependency| {
                pending.contains(&dependency.module_index)
                    || self.module_eager_dependency_is_pending(
                        dependency.module_index,
                        pending,
                        seen,
                    )
            })
    }

    fn push_pending_module_awaits(
        &self,
        statements: &mut Vec<Statement>,
        pending: &mut Vec<usize>,
    ) {
        for module_index in pending.drain(..) {
            self.push_module_async_completion(statements, module_index);
        }
    }

    fn push_module_async_completion(&self, statements: &mut Vec<Statement>, module_index: usize) {
        let module = &self.modules[module_index];
        statements.push(Statement::Expression(Expression::Await(Box::new(
            Expression::Identifier(module.promise_name.clone()),
        ))));
        if module.async_continuation_names.is_empty() {
            return;
        }

        let arguments = self.module_init_call_arguments(module_index);
        for continuation_name in &module.async_continuation_names {
            let continuation_call = Expression::Call {
                callee: Box::new(Expression::Identifier(continuation_name.clone())),
                arguments: arguments.clone(),
            };
            let completion = if self.module_async_continuation_is_async(continuation_name) {
                Expression::Await(Box::new(continuation_call))
            } else {
                continuation_call
            };
            statements.push(Statement::Expression(completion));
        }
    }

    fn module_async_continuation_is_async(&self, continuation_name: &str) -> bool {
        self.lowerer
            .functions
            .iter()
            .rev()
            .find(|function| function.name == continuation_name)
            .is_some_and(|function| function.kind.is_async())
    }

    fn push_module_init_evaluation(
        &self,
        statements: &mut Vec<Statement>,
        pending_deferred_async: &mut Vec<usize>,
        module_index: usize,
        defer_async_completion: bool,
    ) {
        let module = &self.modules[module_index];
        let init_call = Expression::Call {
            callee: Box::new(Expression::Identifier(module.init_name.clone())),
            arguments: self.module_init_call_arguments(module_index),
        };
        if module.init_async {
            statements.push(Statement::Let {
                name: module.promise_name.clone(),
                mutable: false,
                value: init_call,
            });
            if defer_async_completion {
                pending_deferred_async.push(module_index);
            } else {
                self.push_module_async_completion(statements, module_index);
            }
        } else {
            statements.push(Statement::Expression(init_call));
        }
    }

    pub(super) fn bundle_statements(&self, entry_index: usize) -> Result<Vec<Statement>> {
        let mut statements = self.module_registry_statements();
        let mut pending_deferred_async = Vec::new();
        let mut delayed_deferred_async = Vec::new();

        for &module_index in &self.load_order {
            if module_index == entry_index {
                self.push_pending_module_awaits(&mut statements, &mut pending_deferred_async);
                while !delayed_deferred_async.is_empty() {
                    let delayed = std::mem::take(&mut delayed_deferred_async);
                    for delayed_module_index in delayed {
                        let pending_set = pending_deferred_async
                            .iter()
                            .copied()
                            .collect::<HashSet<_>>();
                        if self.module_eager_dependency_is_pending(
                            delayed_module_index,
                            &pending_set,
                            &mut HashSet::new(),
                        ) {
                            delayed_deferred_async.push(delayed_module_index);
                            continue;
                        }
                        self.push_module_init_evaluation(
                            &mut statements,
                            &mut pending_deferred_async,
                            delayed_module_index,
                            true,
                        );
                    }
                    self.push_pending_module_awaits(&mut statements, &mut pending_deferred_async);
                }
            } else {
                let pending_set = pending_deferred_async
                    .iter()
                    .copied()
                    .collect::<HashSet<_>>();
                if self.module_eager_dependency_is_pending(
                    module_index,
                    &pending_set,
                    &mut HashSet::new(),
                ) {
                    delayed_deferred_async.push(module_index);
                    continue;
                }
            }

            self.push_module_init_evaluation(
                &mut statements,
                &mut pending_deferred_async,
                module_index,
                module_index != entry_index && self.modules[module_index].init_async,
            );
        }

        Ok(statements)
    }

    pub(super) fn rewrite_import_bindings_in_statements(
        &self,
        statements: &mut [Statement],
        import_bindings: &HashMap<String, ImportBinding>,
    ) -> Result<()> {
        let mut rewriter = import_rewriter::ImportBindingRewriter::new(import_bindings);
        rewriter.rewrite_statement_list(statements)
    }

    pub(super) fn rewrite_module_import_bindings_in_statements(
        &self,
        module_index: usize,
        statements: &mut [Statement],
        import_bindings: &HashMap<String, ImportBinding>,
    ) -> Result<()> {
        let mut rewriter =
            import_rewriter::ImportBindingRewriter::new_for_module(import_bindings, module_index);
        rewriter.rewrite_statement_list(statements)
    }
}
