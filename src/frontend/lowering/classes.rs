use super::*;

const INSTANCE_FIELD_INITIALIZER_LABEL: &str = "__ayy_instance_field_initializers";
const NULL_SUPER_CONSTRUCTOR_BINDING: &str = "__ayy_null_super_constructor";

impl Lowerer {
    pub(crate) fn lower_class_declaration(
        &mut self,
        class_declaration: &ClassDecl,
    ) -> Result<Vec<Statement>> {
        let source_name = class_declaration.ident.sym.to_string();
        let outer_name = self.resolve_binding_name(&source_name);
        self.push_binding_scope(vec![source_name.clone()]);
        let inner_name = self.fresh_isolated_binding_name(&source_name);
        if let Some(scope) = self.binding_scopes.last_mut() {
            scope
                .renames
                .insert(source_name.clone(), inner_name.clone());
        }
        let mut body = self.lower_class_definition_with_mode(
            &class_declaration.class,
            inner_name.clone(),
            false,
        )?;
        self.pop_binding_scope();
        if !self.class_has_explicit_static_name_property(&class_declaration.class) {
            body.push(define_property_statement(
                Expression::Identifier(inner_name.clone()),
                Expression::String("name".to_string()),
                data_property_descriptor(
                    Expression::String(source_name.clone()),
                    false,
                    false,
                    true,
                ),
            ));
        }
        body.push(Statement::Let {
            name: outer_name,
            mutable: true,
            value: Expression::Identifier(inner_name),
        });
        Ok(vec![Statement::Declaration { body }])
    }

    pub(crate) fn lower_generator_class_declaration(
        &mut self,
        class_declaration: &ClassDecl,
    ) -> Result<Vec<Statement>> {
        let source_name = class_declaration.ident.sym.to_string();
        let outer_name = self.resolve_binding_name(&source_name);
        self.push_binding_scope(vec![source_name.clone()]);
        let inner_name = self.fresh_isolated_binding_name(&source_name);
        if let Some(scope) = self.binding_scopes.last_mut() {
            scope
                .renames
                .insert(source_name.clone(), inner_name.clone());
        }
        let mut body = self.lower_class_definition_with_mode(
            &class_declaration.class,
            inner_name.clone(),
            true,
        )?;
        self.pop_binding_scope();
        if !self.class_has_explicit_static_name_property(&class_declaration.class) {
            body.push(define_property_statement(
                Expression::Identifier(inner_name.clone()),
                Expression::String("name".to_string()),
                data_property_descriptor(
                    Expression::String(source_name.clone()),
                    false,
                    false,
                    true,
                ),
            ));
        }
        body.push(Statement::Let {
            name: outer_name,
            mutable: true,
            value: Expression::Identifier(inner_name),
        });
        Ok(vec![Statement::Declaration { body }])
    }

    pub(crate) fn lower_class_expression(
        &mut self,
        class_expression: &swc_ecma_ast::ClassExpr,
        name_hint: Option<&str>,
    ) -> Result<Expression> {
        let explicit_name = class_expression
            .ident
            .as_ref()
            .map(|identifier| identifier.sym.to_string());
        let pushed_scope = explicit_name.is_some();
        if let Some(explicit_name) = explicit_name.as_ref() {
            self.push_binding_scope(vec![explicit_name.clone()]);
            let scoped_name = self.fresh_isolated_binding_name(explicit_name);
            if let Some(scope) = self.binding_scopes.last_mut() {
                scope.renames.insert(explicit_name.clone(), scoped_name);
            }
        }
        let class_name = explicit_name
            .as_ref()
            .map(|name| self.resolve_binding_name(name))
            .unwrap_or_else(|| self.fresh_temporary_name("class_expr"));
        let display_name = explicit_name
            .or_else(|| name_hint.map(str::to_string))
            .unwrap_or_default();
        let init_name = self.fresh_temporary_name("class_init");
        let init_body_result = self.lower_class_definition_with_mode(
            &class_expression.class,
            class_name.clone(),
            false,
        );
        if pushed_scope {
            self.pop_binding_scope();
        }
        let mut init_body = init_body_result?;
        if !self.class_has_explicit_static_name_property(&class_expression.class) {
            init_body.push(define_property_statement(
                Expression::Identifier(class_name.clone()),
                Expression::String("name".to_string()),
                data_property_descriptor(Expression::String(display_name), false, false, true),
            ));
        }
        init_body.push(Statement::Return(Expression::Identifier(class_name)));

        self.functions.push(FunctionDeclaration {
            name: init_name.clone(),
            top_level_binding: None,
            params: Vec::new(),
            body: init_body,
            register_global: false,
            kind: FunctionKind::Ordinary,
            self_binding: None,
            mapped_arguments: false,
            strict: true,
            lexical_this: false,
            derived_constructor: false,
            direct_eval_in_class_field_initializer: self.class_field_initializer_depth > 0,
            length: 0,
            synthetic_capture_bindings: Vec::new(),
            immutable_class_bindings: Vec::new(),
            private_brand_binding: None,
        });

        Ok(Expression::Call {
            callee: Box::new(Expression::Identifier(init_name)),
            arguments: Vec::new(),
        })
    }

    pub(crate) fn lower_generator_class_expression(
        &mut self,
        class_expression: &swc_ecma_ast::ClassExpr,
        name_hint: Option<&str>,
    ) -> Result<(Vec<Statement>, Expression)> {
        let explicit_name = class_expression
            .ident
            .as_ref()
            .map(|identifier| identifier.sym.to_string());
        let pushed_scope = explicit_name.is_some();
        if let Some(explicit_name) = explicit_name.as_ref() {
            self.push_binding_scope(vec![explicit_name.clone()]);
            let scoped_name = self.fresh_isolated_binding_name(explicit_name);
            if let Some(scope) = self.binding_scopes.last_mut() {
                scope.renames.insert(explicit_name.clone(), scoped_name);
            }
        }
        let class_name = explicit_name
            .as_ref()
            .map(|name| self.resolve_binding_name(name))
            .unwrap_or_else(|| self.fresh_temporary_name("class_expr"));
        let display_name = explicit_name
            .or_else(|| name_hint.map(str::to_string))
            .unwrap_or_default();

        let statements_result = self.lower_class_definition_with_mode(
            &class_expression.class,
            class_name.clone(),
            true,
        );
        if pushed_scope {
            self.pop_binding_scope();
        }
        let mut statements = statements_result?;
        if !self.class_has_explicit_static_name_property(&class_expression.class) {
            statements.push(define_property_statement(
                Expression::Identifier(class_name.clone()),
                Expression::String("name".to_string()),
                data_property_descriptor(Expression::String(display_name), false, false, true),
            ));
        }

        Ok((statements, Expression::Identifier(class_name)))
    }

    fn class_has_explicit_static_name_property(&self, class: &Class) -> bool {
        class.body.iter().any(|member| match member {
            ClassMember::ClassProp(property) => {
                property.is_static && self.class_prop_name_is_name(&property.key)
            }
            ClassMember::Method(method) => {
                method.is_static && self.class_prop_name_is_name(&method.key)
            }
            ClassMember::AutoAccessor(accessor) => match &accessor.key {
                Key::Public(property) => {
                    accessor.is_static && self.class_prop_name_is_name(property)
                }
                Key::Private(_) => false,
            },
            _ => false,
        })
    }

    fn class_prop_name_is_name(&self, property: &PropName) -> bool {
        match property {
            PropName::Ident(identifier) => identifier.sym == *"name",
            PropName::Str(string) => string.value == *"name",
            PropName::Computed(computed) => {
                matches!(computed.expr.as_ref(), Expr::Lit(Lit::Str(string)) if string.value == *"name")
            }
            _ => false,
        }
    }

    fn class_has_instance_private_members(&self, class: &Class) -> bool {
        class.body.iter().any(|member| match member {
            ClassMember::PrivateProp(property) => !property.is_static,
            ClassMember::PrivateMethod(method) => !method.is_static,
            ClassMember::AutoAccessor(accessor) => {
                !accessor.is_static && matches!(accessor.key, Key::Private(_))
            }
            _ => false,
        })
    }

    fn lower_class_field_initializer_value(
        &mut self,
        value: &Expr,
        name_hint: Option<&str>,
        this_replacement: Option<Expression>,
    ) -> Result<Expression> {
        self.class_field_initializer_depth += 1;
        let lowered = self.with_this_replacement(this_replacement, |lowerer| {
            lowerer.lower_expression_with_name_hint(value, name_hint)
        });
        self.class_field_initializer_depth -= 1;
        lowered
    }

    pub(crate) fn lower_class_definition_with_mode(
        &mut self,
        class: &Class,
        binding_name: String,
        generator_body: bool,
    ) -> Result<Vec<Statement>> {
        let instance_private_brand_binding = self
            .class_has_instance_private_members(class)
            .then(|| self.fresh_temporary_name("class_brand"));
        self.private_name_scopes
            .push(self.class_private_name_map(class, &binding_name));
        self.private_name_brand_scopes
            .push(self.class_private_brand_map(class, instance_private_brand_binding.as_deref()));
        let class_identifier = Expression::Identifier(binding_name.clone());
        let extends_null = matches!(class.super_class.as_deref(), Some(Expr::Lit(Lit::Null(_))));
        let super_name = class
            .super_class
            .as_ref()
            .filter(|_| !extends_null)
            .map(|_| self.fresh_temporary_name("class_super"));
        let constructor_name = self.lower_class_constructor(
            class,
            &binding_name,
            super_name.as_deref(),
            extends_null,
            instance_private_brand_binding.as_deref(),
        )?;
        let prototype_parent = if extends_null {
            Expression::Null
        } else {
            super_name
                .as_ref()
                .map(|name| Expression::Member {
                    object: Box::new(Expression::Identifier(name.clone())),
                    property: Box::new(Expression::String("prototype".to_string())),
                })
                .unwrap_or(Expression::Member {
                    object: Box::new(Expression::Identifier("Object".to_string())),
                    property: Box::new(Expression::String("prototype".to_string())),
                })
        };
        let prototype_target = Expression::Member {
            object: Box::new(class_identifier.clone()),
            property: Box::new(Expression::String("prototype".to_string())),
        };

        let mut statements = Vec::new();
        let mut instance_field_initializers = Vec::new();
        if let (Some(super_expression), Some(super_name)) =
            (&class.super_class, super_name.as_ref())
        {
            self.strict_modes.push(true);
            let lowered_super_expression = self.lower_expression(super_expression);
            self.strict_modes.pop();
            statements.push(Statement::Let {
                name: super_name.clone(),
                mutable: false,
                value: lowered_super_expression?,
            });
        }
        if let Some(instance_private_brand_binding) = instance_private_brand_binding.as_ref() {
            statements.push(Statement::Let {
                name: instance_private_brand_binding.clone(),
                mutable: false,
                value: Expression::Object(Vec::new()),
            });
        }

        statements.extend([
            Statement::Let {
                name: binding_name.clone(),
                mutable: Self::scoped_class_expression_source_name(&binding_name).is_none(),
                value: Expression::Identifier(constructor_name.clone()),
            },
            define_property_statement(
                class_identifier.clone(),
                Expression::String("name".to_string()),
                data_property_descriptor(
                    Expression::String(binding_name.clone()),
                    false,
                    false,
                    true,
                ),
            ),
            Statement::Expression(Expression::Call {
                callee: Box::new(Expression::Identifier(
                    "__ayyClassPrototypeInit".to_string(),
                )),
                arguments: vec![
                    CallArgument::Expression(class_identifier.clone()),
                    CallArgument::Expression(prototype_parent),
                ],
            }),
            define_property_statement(
                prototype_target.clone(),
                Expression::String("constructor".to_string()),
                data_property_descriptor(class_identifier.clone(), true, false, true),
            ),
        ]);
        if let Some(super_name) = super_name.as_ref() {
            statements.push(Statement::Expression(Expression::Call {
                callee: Box::new(Expression::Member {
                    object: Box::new(Expression::Identifier("Object".to_string())),
                    property: Box::new(Expression::String("setPrototypeOf".to_string())),
                }),
                arguments: vec![
                    CallArgument::Expression(class_identifier.clone()),
                    CallArgument::Expression(Expression::Identifier(super_name.clone())),
                ],
            }));
        }

        for member in &class.body {
            match member {
                ClassMember::ClassProp(property) => {
                    let name_hint = self.class_prop_name_hint(&property.key);
                    let (mut property_prefix, lowered_property_name) =
                        self.lower_class_prop_name(&property.key, generator_body)?;
                    statements.append(&mut property_prefix);
                    let property_name = match &property.key {
                        PropName::Computed(_) => {
                            let computed_name = self.fresh_temporary_name("class_field_name");
                            statements.push(Statement::Let {
                                name: computed_name.clone(),
                                mutable: false,
                                value: lowered_property_name,
                            });
                            Expression::Identifier(computed_name)
                        }
                        _ => lowered_property_name,
                    };
                    let value = property
                        .value
                        .as_ref()
                        .map(|value| {
                            self.lower_class_field_initializer_value(
                                value,
                                name_hint.as_deref(),
                                property.is_static.then(|| class_identifier.clone()),
                            )
                        })
                        .transpose()?
                        .unwrap_or(Expression::Undefined);
                    if property.is_static {
                        statements.push(define_property_statement(
                            class_identifier.clone(),
                            property_name,
                            data_property_descriptor(value, true, true, true),
                        ));
                    } else {
                        instance_field_initializers.push(define_property_statement(
                            Expression::This,
                            property_name,
                            data_property_descriptor(value, true, true, true),
                        ));
                    }
                }
                ClassMember::PrivateProp(property) => {
                    let name_hint = format!("#{}", property.key.name);
                    let value = property
                        .value
                        .as_ref()
                        .map(|value| {
                            self.lower_class_field_initializer_value(
                                value,
                                Some(&name_hint),
                                property.is_static.then(|| class_identifier.clone()),
                            )
                        })
                        .transpose()?
                        .unwrap_or(Expression::Undefined);
                    let target = if property.is_static {
                        class_identifier.clone()
                    } else {
                        Expression::This
                    };
                    let initializer = Statement::AssignMember {
                        object: target,
                        property: self.lower_private_name(&property.key)?,
                        value,
                    };
                    if property.is_static {
                        statements.push(Self::instance_field_initializer_block(vec![initializer]));
                    } else {
                        instance_field_initializers.push(initializer);
                    }
                }
                ClassMember::AutoAccessor(accessor) => match &accessor.key {
                    Key::Public(property_key) => {
                        let name_hint = self.class_prop_name_hint(property_key);
                        let (mut property_prefix, lowered_property_name) =
                            self.lower_class_prop_name(property_key, generator_body)?;
                        statements.append(&mut property_prefix);
                        let property_name = match property_key {
                            PropName::Computed(_) => {
                                let computed_name = self.fresh_temporary_name("class_field_name");
                                statements.push(Statement::Let {
                                    name: computed_name.clone(),
                                    mutable: false,
                                    value: lowered_property_name,
                                });
                                Expression::Identifier(computed_name)
                            }
                            _ => lowered_property_name,
                        };
                        let value = accessor
                            .value
                            .as_ref()
                            .map(|value| {
                                self.lower_class_field_initializer_value(
                                    value,
                                    name_hint.as_deref(),
                                    accessor.is_static.then(|| class_identifier.clone()),
                                )
                            })
                            .transpose()?
                            .unwrap_or(Expression::Undefined);
                        if accessor.is_static {
                            statements.push(define_property_statement(
                                class_identifier.clone(),
                                property_name,
                                data_property_descriptor(value, true, true, true),
                            ));
                        } else {
                            instance_field_initializers.push(define_property_statement(
                                Expression::This,
                                property_name,
                                data_property_descriptor(value, true, true, true),
                            ));
                        }
                    }
                    Key::Private(private_name) => {
                        let name_hint = format!("#{}", private_name.name);
                        let value = accessor
                            .value
                            .as_ref()
                            .map(|value| {
                                self.lower_class_field_initializer_value(
                                    value,
                                    Some(&name_hint),
                                    accessor.is_static.then(|| class_identifier.clone()),
                                )
                            })
                            .transpose()?
                            .unwrap_or(Expression::Undefined);
                        let target = if accessor.is_static {
                            class_identifier.clone()
                        } else {
                            Expression::This
                        };
                        let initializer = Statement::AssignMember {
                            object: target,
                            property: self.lower_private_name(private_name)?,
                            value,
                        };
                        if accessor.is_static {
                            statements
                                .push(Self::instance_field_initializer_block(vec![initializer]));
                        } else {
                            instance_field_initializers.push(initializer);
                        }
                    }
                },
                ClassMember::PrivateMethod(method) => {
                    let property = self.lower_private_name(&method.key)?;
                    let display_name = self.private_method_display_name(&method.key, method.kind);
                    let private_brand_binding = (!method.is_static)
                        .then_some(instance_private_brand_binding.as_deref())
                        .flatten();
                    let lowered_method_name = self.lower_class_method_function(
                        &method.function,
                        Some(&display_name),
                        &binding_name,
                        private_brand_binding,
                    )?;
                    let descriptor = match method.kind {
                        MethodKind::Method => data_property_descriptor(
                            Expression::Identifier(lowered_method_name.clone()),
                            true,
                            false,
                            true,
                        ),
                        MethodKind::Getter => getter_property_descriptor(
                            Expression::Identifier(lowered_method_name.clone()),
                            false,
                            true,
                        ),
                        MethodKind::Setter => setter_property_descriptor(
                            Expression::Identifier(lowered_method_name.clone()),
                            false,
                            true,
                        ),
                    };
                    let target = if method.is_static {
                        class_identifier.clone()
                    } else {
                        prototype_target.clone()
                    };
                    if method.is_static {
                        statements.push(Self::instance_field_initializer_block(
                            self.lower_static_class_method_definition(
                                target,
                                property.clone(),
                                descriptor,
                            ),
                        ));
                    } else {
                        statements.push(define_property_statement(
                            target,
                            property.clone(),
                            descriptor,
                        ));
                    }
                    if let Some(private_brand_binding) = private_brand_binding {
                        let marker = if method.kind == MethodKind::Method {
                            Expression::Identifier(lowered_method_name)
                        } else {
                            Expression::Identifier(private_brand_binding.to_string())
                        };
                        let marker_initializer = if method.kind == MethodKind::Method {
                            Statement::AssignMember {
                                object: Expression::This,
                                property,
                                value: marker,
                            }
                        } else {
                            define_property_statement(
                                Expression::This,
                                property,
                                data_property_descriptor(marker, true, false, true),
                            )
                        };
                        instance_field_initializers.push(marker_initializer);
                    }
                }
                _ => {
                    statements.extend(self.lower_class_member_with_mode(
                        class,
                        member,
                        &binding_name,
                        &prototype_target,
                        instance_private_brand_binding.as_deref(),
                        super_name.as_deref(),
                        generator_body,
                    )?);
                }
            }
        }

        if !instance_field_initializers.is_empty() {
            let constructor = self
                .functions
                .iter_mut()
                .rfind(|function| function.name == constructor_name)
                .context(
                    "lowered class constructor should exist for public field initialization",
                )?;
            Self::insert_instance_field_initializers(
                constructor,
                instance_field_initializers,
                super_name.is_some(),
            )?;
        }

        self.private_name_brand_scopes.pop();
        self.private_name_scopes.pop();

        Ok(statements)
    }

    pub(crate) fn lower_class_constructor(
        &mut self,
        class: &Class,
        binding_name: &str,
        super_name: Option<&str>,
        extends_null: bool,
        private_brand_binding: Option<&str>,
    ) -> Result<String> {
        let constructor = class.body.iter().find_map(|member| match member {
            ClassMember::Constructor(constructor) => Some(constructor),
            _ => None,
        });

        let generated_name = format!(
            "__ayy_class_ctor_{}__name_{}",
            self.fresh_temporary_name("ctor"),
            binding_name
        );

        let (params, param_setup, body, length) = if let Some(constructor) = constructor {
            self.with_this_replacement(None, |lowerer| {
                lowerer.with_super_member_replacement(None, |lowerer| {
                    let (params, param_setup, length) =
                        lower_constructor_parameters(lowerer, constructor)?;
                    let body = if let Some(body) = &constructor.body {
                        lowerer.constructor_super_stack.push(if extends_null {
                            Some(NULL_SUPER_CONSTRUCTOR_BINDING.to_string())
                        } else {
                            super_name.map(ToOwned::to_owned)
                        });
                        lowerer.strict_modes.push(true);
                        let lowered = lowerer.lower_statements(&body.stmts, true, false);
                        lowerer.strict_modes.pop();
                        lowerer.constructor_super_stack.pop();
                        lowered?
                    } else {
                        Vec::new()
                    };
                    Ok((params, param_setup, body, length))
                })
            })?
        } else {
            self.lower_default_class_constructor_body(super_name, extends_null)
        };

        let mut body = body;
        body.insert(
            0,
            Statement::If {
                condition: Expression::Binary {
                    op: BinaryOp::Equal,
                    left: Box::new(Expression::NewTarget),
                    right: Box::new(Expression::Undefined),
                },
                then_branch: vec![Statement::Throw(Expression::New {
                    callee: Box::new(Expression::Identifier("TypeError".to_string())),
                    arguments: Vec::new(),
                })],
                else_branch: Vec::new(),
            },
        );
        body.splice(0..0, param_setup);

        self.functions.push(FunctionDeclaration {
            name: generated_name.clone(),
            top_level_binding: None,
            params,
            body,
            register_global: false,
            kind: FunctionKind::Ordinary,
            self_binding: Some(binding_name.to_string()),
            mapped_arguments: false,
            strict: true,
            lexical_this: false,
            derived_constructor: super_name.is_some() || extends_null,
            direct_eval_in_class_field_initializer: self.class_field_initializer_depth > 0,
            length,
            synthetic_capture_bindings: private_brand_binding
                .into_iter()
                .chain(super_name)
                .map(str::to_string)
                .collect(),
            immutable_class_bindings: vec![binding_name.to_string()],
            private_brand_binding: private_brand_binding.map(str::to_string),
        });

        Ok(generated_name)
    }

    fn lower_default_class_constructor_body(
        &mut self,
        super_name: Option<&str>,
        extends_null: bool,
    ) -> (Vec<Parameter>, Vec<Statement>, Vec<Statement>, usize) {
        let super_name = if extends_null {
            NULL_SUPER_CONSTRUCTOR_BINDING
        } else if let Some(super_name) = super_name {
            super_name
        } else {
            return (Vec::new(), Vec::new(), Vec::new(), 0);
        };
        let args_name = self.fresh_temporary_name("args");
        (
            vec![Parameter {
                name: args_name.clone(),
                default: None,
                rest: true,
            }],
            Vec::new(),
            vec![Statement::Expression(Expression::SuperCall {
                callee: Box::new(Expression::Identifier(super_name.to_string())),
                arguments: vec![CallArgument::Spread(Expression::Identifier(args_name))],
            })],
            0,
        )
    }

    fn insert_instance_field_initializers(
        constructor: &mut FunctionDeclaration,
        initializers: Vec<Statement>,
        derived_constructor: bool,
    ) -> Result<()> {
        if initializers.is_empty() {
            return Ok(());
        }
        let initializers = Self::instance_field_initializer_block(initializers);
        constructor.direct_eval_in_class_field_initializer = true;
        if !derived_constructor {
            constructor.body.insert(0, initializers);
            return Ok(());
        }

        let mut initializers = Some(initializers);
        let mut index = 0;
        while index < constructor.body.len() {
            match &constructor.body[index] {
                Statement::Expression(Expression::SuperCall { .. })
                | Statement::Var {
                    value: Expression::SuperCall { .. },
                    ..
                }
                | Statement::Let {
                    value: Expression::SuperCall { .. },
                    ..
                }
                | Statement::Assign {
                    value: Expression::SuperCall { .. },
                    ..
                } => {
                    constructor
                        .body
                        .insert(index + 1, initializers.take().unwrap());
                    return Ok(());
                }
                Statement::Return(Expression::SuperCall { .. }) => {
                    let Statement::Return(super_result) = constructor.body.remove(index) else {
                        unreachable!("filtered return super() above");
                    };
                    let super_result_name = "__ayy_super_result".to_string();
                    let mut replacement = vec![Statement::Let {
                        name: super_result_name.clone(),
                        mutable: false,
                        value: super_result,
                    }];
                    replacement.push(initializers.take().unwrap());
                    replacement.push(Statement::Return(Expression::Identifier(super_result_name)));
                    constructor.body.splice(index..index, replacement);
                    return Ok(());
                }
                _ => {
                    index += 1;
                }
            }
        }

        constructor.body.push(initializers.take().unwrap());
        Ok(())
    }

    fn instance_field_initializer_block(initializers: Vec<Statement>) -> Statement {
        Statement::Labeled {
            labels: vec![INSTANCE_FIELD_INITIALIZER_LABEL.to_string()],
            body: initializers,
        }
    }

    pub(crate) fn lower_class_member_with_mode(
        &mut self,
        class: &Class,
        member: &ClassMember,
        class_name: &str,
        prototype_target: &Expression,
        instance_private_brand_binding: Option<&str>,
        static_super_name: Option<&str>,
        generator_body: bool,
    ) -> Result<Vec<Statement>> {
        match member {
            ClassMember::Constructor(_) | ClassMember::Empty(_) | ClassMember::PrivateProp(_) => {
                Ok(Vec::new())
            }
            ClassMember::StaticBlock(block) => {
                self.strict_modes.push(true);
                let super_replacement =
                    static_super_name.map(|name| Expression::Identifier(name.to_string()));
                let lowered = self.with_this_replacement(
                    Some(Expression::Identifier(class_name.to_string())),
                    |lowerer| {
                        lowerer.with_super_member_replacement(super_replacement, |lowerer| {
                            lowerer.lower_static_block_statements(&block.body.stmts)
                        })
                    },
                );
                self.strict_modes.pop();
                lowered
            }
            ClassMember::Method(method) => {
                let (mut prefix, property) =
                    self.lower_class_prop_name(&method.key, generator_body)?;
                let target = if method.is_static {
                    Expression::Identifier(class_name.to_string())
                } else {
                    prototype_target.clone()
                };
                if method.kind == MethodKind::Getter {
                    if let Some(private_alias) =
                        self.lower_private_method_alias_getter(class, method, &target)?
                    {
                        prefix.push(define_property_statement(
                            target,
                            property,
                            data_property_descriptor(private_alias, false, false, true),
                        ));
                        return Ok(prefix);
                    }
                }
                prefix.extend(
                    self.lower_defined_class_method(
                        class_name,
                        prototype_target,
                        method.is_static,
                        method.kind,
                        property,
                        None,
                        (!method.is_static)
                            .then_some(instance_private_brand_binding)
                            .flatten(),
                        &method.function,
                    )?,
                );
                Ok(prefix)
            }
            other => bail!("unsupported class member: {other:?}"),
        }
    }

    fn lower_class_prop_name(
        &mut self,
        name: &PropName,
        generator_body: bool,
    ) -> Result<(Vec<Statement>, Expression)> {
        if !generator_body {
            return Ok((Vec::new(), self.lower_prop_name(name)?));
        }

        Ok(match name {
            PropName::Ident(identifier) => {
                (Vec::new(), Expression::String(identifier.sym.to_string()))
            }
            PropName::Str(string) => (
                Vec::new(),
                Expression::String(string.value.to_string_lossy().into_owned()),
            ),
            PropName::Num(number) => (Vec::new(), Expression::Number(number.value)),
            PropName::Computed(computed) => {
                if let Some((prefix, value)) =
                    self.lower_generator_assignment_value(&computed.expr)?
                {
                    (prefix, value)
                } else {
                    (Vec::new(), self.lower_expression(&computed.expr)?)
                }
            }
            _ => bail!("unsupported object property key"),
        })
    }

    fn class_prop_name_hint(&self, name: &PropName) -> Option<String> {
        match name {
            PropName::Ident(identifier) => Some(identifier.sym.to_string()),
            PropName::Str(string) => Some(string.value.to_string_lossy().into_owned()),
            PropName::Num(number) => Some(number.value.to_string()),
            PropName::Computed(computed) => match computed.expr.as_ref() {
                Expr::Lit(Lit::Str(string)) => Some(string.value.to_string_lossy().into_owned()),
                Expr::Lit(Lit::Num(number)) => Some(number.value.to_string()),
                Expr::Lit(Lit::BigInt(bigint)) => Some(bigint.value.to_string()),
                Expr::Lit(Lit::Bool(boolean)) => Some(boolean.value.to_string()),
                Expr::Lit(Lit::Null(_)) => Some("null".to_string()),
                _ => None,
            },
            _ => None,
        }
    }

    fn private_getter_aliases_private_method(
        &self,
        class: &Class,
        is_static: bool,
        private_name: &swc_ecma_ast::PrivateName,
    ) -> bool {
        class.body.iter().any(|member| {
            matches!(
                member,
                ClassMember::PrivateMethod(method)
                    if method.is_static == is_static
                        && method.kind == MethodKind::Method
                        && method.key.name == private_name.name
            )
        })
    }

    pub(crate) fn lower_private_method_alias_getter(
        &mut self,
        class: &Class,
        method: &ClassMethod,
        target: &Expression,
    ) -> Result<Option<Expression>> {
        let Some(body) = method.function.body.as_ref() else {
            return Ok(None);
        };
        if !method.function.params.is_empty() || body.stmts.len() != 1 {
            return Ok(None);
        }
        let swc_ecma_ast::Stmt::Return(return_statement) = &body.stmts[0] else {
            return Ok(None);
        };
        let Some(return_value) = return_statement.arg.as_deref() else {
            return Ok(None);
        };
        let Expr::Member(member) = return_value else {
            return Ok(None);
        };
        if !matches!(member.obj.as_ref(), Expr::This(_)) {
            return Ok(None);
        }
        let MemberProp::PrivateName(private_name) = &member.prop else {
            return Ok(None);
        };
        if !self.private_getter_aliases_private_method(class, method.is_static, private_name) {
            return Ok(None);
        }
        Ok(Some(Expression::Member {
            object: Box::new(target.clone()),
            property: Box::new(self.lower_private_name(private_name)?),
        }))
    }

    pub(crate) fn lower_defined_class_method(
        &mut self,
        class_name: &str,
        prototype_target: &Expression,
        is_static: bool,
        kind: MethodKind,
        property: Expression,
        display_name_hint: Option<&str>,
        private_brand_binding: Option<&str>,
        function: &Function,
    ) -> Result<Vec<Statement>> {
        let target = if is_static {
            Expression::Identifier(class_name.to_string())
        } else {
            prototype_target.clone()
        };
        let descriptor = match kind {
            MethodKind::Method => {
                let method_name = self.lower_class_method_function(
                    function,
                    display_name_hint,
                    class_name,
                    private_brand_binding,
                )?;
                data_property_descriptor(Expression::Identifier(method_name), true, false, true)
            }
            MethodKind::Getter => {
                let getter_name = self.lower_class_method_function(
                    function,
                    display_name_hint,
                    class_name,
                    private_brand_binding,
                )?;
                getter_property_descriptor(Expression::Identifier(getter_name), false, true)
            }
            MethodKind::Setter => {
                let setter_name = self.lower_class_method_function(
                    function,
                    display_name_hint,
                    class_name,
                    private_brand_binding,
                )?;
                setter_property_descriptor(Expression::Identifier(setter_name), false, true)
            }
        };

        if is_static {
            return Ok(self.lower_static_class_method_definition(target, property, descriptor));
        }

        Ok(vec![define_property_statement(
            target, property, descriptor,
        )])
    }

    pub(crate) fn lower_static_class_method_definition(
        &mut self,
        target: Expression,
        property: Expression,
        descriptor: Expression,
    ) -> Vec<Statement> {
        if matches!(&property, Expression::String(name) if name == "prototype") {
            return vec![Statement::Throw(Expression::New {
                callee: Box::new(Expression::Identifier("TypeError".to_string())),
                arguments: Vec::new(),
            })];
        }

        if matches!(
            property,
            Expression::String(_)
                | Expression::Number(_)
                | Expression::BigInt(_)
                | Expression::Bool(_)
                | Expression::Null
                | Expression::Undefined
        ) {
            return vec![define_property_statement(target, property, descriptor)];
        }

        let property_name = self.fresh_temporary_name("class_prop");
        let property_identifier = Expression::Identifier(property_name.clone());

        vec![
            Statement::Let {
                name: property_name,
                mutable: false,
                value: property,
            },
            Statement::If {
                condition: Expression::Binary {
                    op: BinaryOp::Equal,
                    left: Box::new(property_identifier.clone()),
                    right: Box::new(Expression::String("prototype".to_string())),
                },
                then_branch: vec![Statement::Throw(Expression::New {
                    callee: Box::new(Expression::Identifier("TypeError".to_string())),
                    arguments: Vec::new(),
                })],
                else_branch: vec![define_property_statement(
                    target,
                    property_identifier,
                    descriptor,
                )],
            },
        ]
    }

    fn private_method_display_name(
        &self,
        private_name: &swc_ecma_ast::PrivateName,
        kind: MethodKind,
    ) -> String {
        let base_name = format!("#{}", private_name.name);
        match kind {
            MethodKind::Method => base_name,
            MethodKind::Getter => format!("get {base_name}"),
            MethodKind::Setter => format!("set {base_name}"),
        }
    }

    fn scoped_class_expression_source_name(class_binding_name: &str) -> Option<String> {
        class_binding_name
            .strip_prefix("__ayy_scope$")
            .and_then(|name| name.rsplit_once('$').map(|(source_name, _)| source_name))
            .map(str::to_string)
    }

    pub(crate) fn lower_class_method_function(
        &mut self,
        function: &Function,
        name_hint: Option<&str>,
        class_binding_name: &str,
        private_brand_binding: Option<&str>,
    ) -> Result<String> {
        self.next_function_expression_id += 1;
        let generated_name = match name_hint {
            Some(name_hint) => format!(
                "__ayy_class_method_{}__name_{}",
                self.next_function_expression_id, name_hint
            ),
            None => format!("__ayy_class_method_{}", self.next_function_expression_id),
        };
        self.strict_modes.push(true);
        let (params, body, mut captured_private_brand_bindings) =
            self.lower_function_parts(function, &[])?;
        self.strict_modes.pop();
        if let Some(private_brand_binding) = private_brand_binding {
            captured_private_brand_bindings.remove(private_brand_binding);
        }
        let synthetic_capture_bindings = private_brand_binding
            .into_iter()
            .map(str::to_string)
            .chain(captured_private_brand_bindings)
            .collect::<Vec<_>>();

        self.functions.push(FunctionDeclaration {
            name: generated_name.clone(),
            top_level_binding: None,
            params,
            body,
            register_global: false,
            kind: lower_function_kind(function.is_generator, function.is_async),
            self_binding: None,
            mapped_arguments: false,
            strict: true,
            lexical_this: false,
            derived_constructor: false,
            direct_eval_in_class_field_initializer: self.class_field_initializer_depth > 0,
            length: expected_argument_count(function.params.iter().map(|parameter| &parameter.pat)),
            synthetic_capture_bindings,
            immutable_class_bindings: vec![class_binding_name.to_string()],
            private_brand_binding: private_brand_binding.map(str::to_string),
        });

        Ok(generated_name)
    }
}
