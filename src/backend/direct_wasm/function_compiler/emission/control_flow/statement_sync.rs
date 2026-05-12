use super::*;

impl<'a> FunctionCompiler<'a> {
    fn sync_static_resolution_environment_overrides(
        &mut self,
        environment: &StaticResolutionEnvironment,
    ) {
        for (name, value) in &environment.global_value_overrides {
            self.update_static_global_assignment_metadata(name, value);
        }
        for (name, value) in &environment.local_bindings {
            self.update_local_value_binding(name, value);
            let kind = self
                .infer_value_kind(value)
                .unwrap_or(StaticValueKind::Unknown);
            self.state
                .speculation
                .static_semantics
                .set_local_kind(name, kind);
        }
        for (name, binding) in &environment.local_object_bindings {
            self.state
                .speculation
                .static_semantics
                .set_local_object_binding(name, binding.clone());
            self.state
                .speculation
                .static_semantics
                .set_local_kind(name, StaticValueKind::Object);
        }
        for (name, binding) in &environment.global_object_overrides {
            self.backend
                .sync_global_object_binding(name, binding.clone());
            if binding.is_some() {
                let kind = if self
                    .resolve_function_binding_from_expression(&Expression::Identifier(name.clone()))
                    .is_some()
                {
                    StaticValueKind::Function
                } else {
                    StaticValueKind::Object
                };
                self.backend.set_global_binding_kind(name, kind);
            }
        }
    }

    fn sync_static_binding_tracking_effect(
        &mut self,
        name: &str,
        value: &Expression,
    ) -> DirectResult<()> {
        let mut environment = self.snapshot_static_resolution_environment();
        let evaluated = self
            .evaluate_static_expression_with_state(value, &mut environment)
            .or_else(|| self.materialize_static_expression_with_state(value, &environment))
            .unwrap_or_else(|| self.materialize_static_expression(value));
        self.sync_static_resolution_environment_overrides(&environment);
        if self.binding_name_is_global(name)
            || self.global_has_binding(name)
            || self.global_has_implicit_binding(name)
        {
            self.update_static_global_assignment_metadata(name, &evaluated);
        } else {
            self.update_capture_slot_binding_from_expression(name, &evaluated)?;
        }
        self.update_object_prototype_binding_from_value(name, value);
        self.update_member_function_binding_from_expression(&evaluated);
        self.update_object_binding_from_expression(&evaluated);
        Ok(())
    }

    fn sync_static_define_property_tracking_effect(
        &mut self,
        target: &Expression,
        property: &Expression,
        descriptor_expression: &Expression,
    ) {
        let Some(descriptor) = resolve_property_descriptor_definition(descriptor_expression) else {
            return;
        };

        let mut environment = self.snapshot_static_resolution_environment();
        let property = self
            .evaluate_static_expression_with_state(property, &mut environment)
            .or_else(|| self.materialize_static_expression_with_state(property, &environment))
            .unwrap_or_else(|| self.materialize_static_expression(property));
        let value = if descriptor.is_accessor() {
            Expression::Undefined
        } else {
            descriptor
                .value
                .as_ref()
                .and_then(|value| {
                    self.evaluate_static_expression_with_state(value, &mut environment)
                        .or_else(|| {
                            self.materialize_static_expression_with_state(value, &environment)
                        })
                })
                .unwrap_or(Expression::Undefined)
        };
        self.sync_static_resolution_environment_overrides(&environment);
        let Some(target_name) =
            resolve_stateful_object_binding_name_in_environment(target, &environment).or_else(
                || match target {
                    Expression::Identifier(name) => Some(name.clone()),
                    _ => None,
                },
            )
        else {
            return;
        };
        if !environment.contains_object_binding(&target_name)
            && self
                .resolve_function_binding_from_expression(&Expression::Identifier(
                    target_name.clone(),
                ))
                .is_some()
        {
            if self.binding_name_is_global(&target_name) {
                self.backend
                    .sync_global_object_binding(&target_name, Some(empty_object_value_binding()));
            } else {
                self.state
                    .speculation
                    .static_semantics
                    .set_local_object_binding(&target_name, empty_object_value_binding());
                self.state
                    .speculation
                    .static_semantics
                    .set_local_kind(&target_name, StaticValueKind::Object);
            }
        }
        let property = self
            .resolve_property_key_expression(&property)
            .unwrap_or(property);
        let enumerable = descriptor.enumerable.unwrap_or(false);
        if let Some(binding) = self
            .state
            .speculation
            .static_semantics
            .local_object_binding_mut(&target_name)
        {
            if !object_binding_can_define_property(binding, &property) {
                return;
            }
            object_binding_define_property(binding, property.clone(), value.clone(), enumerable);
            let target_kind = if self
                .resolve_function_binding_from_expression(&Expression::Identifier(
                    target_name.clone(),
                ))
                .is_some()
            {
                StaticValueKind::Function
            } else {
                StaticValueKind::Object
            };
            self.state
                .speculation
                .static_semantics
                .set_local_kind(&target_name, target_kind);
        }
        if self.binding_name_is_global(&target_name)
            || self.backend.global_object_binding(&target_name).is_some()
        {
            let mut binding = self
                .backend
                .global_object_binding(&target_name)
                .cloned()
                .unwrap_or_else(empty_object_value_binding);
            if !object_binding_can_define_property(&binding, &property) {
                return;
            }
            object_binding_define_property(&mut binding, property, value, enumerable);
            self.backend
                .sync_global_object_binding(&target_name, Some(binding));
            let target_kind = if self
                .resolve_function_binding_from_expression(&Expression::Identifier(
                    target_name.clone(),
                ))
                .is_some()
            {
                StaticValueKind::Function
            } else {
                StaticValueKind::Object
            };
            self.backend
                .set_global_binding_kind(&target_name, target_kind);
        }
    }

    fn sync_static_assign_member_tracking_effect(
        &mut self,
        object: &Expression,
        property: &Expression,
        value: &Expression,
    ) {
        let mut environment = self.snapshot_static_resolution_environment();

        let property = self
            .evaluate_static_expression_with_state(property, &mut environment)
            .unwrap_or_else(|| self.materialize_static_expression(property));
        let value = self
            .evaluate_static_expression_with_state(value, &mut environment)
            .unwrap_or_else(|| self.materialize_static_expression(value));
        let Some(target_name) =
            resolve_stateful_object_binding_name_in_environment(object, &environment).or_else(
                || match object {
                    Expression::Identifier(name) => Some(name.clone()),
                    Expression::This => Some("this".to_string()),
                    _ => None,
                },
            )
        else {
            return;
        };
        if !environment.contains_object_binding(&target_name)
            && self
                .resolve_function_binding_from_expression(&Expression::Identifier(
                    target_name.clone(),
                ))
                .is_some()
        {
            environment.set_object_binding(target_name.clone(), empty_object_value_binding());
        }
        let property = self
            .resolve_property_key_expression(&property)
            .unwrap_or(property);
        let Some(binding) = environment.object_binding_mut(&target_name) else {
            return;
        };
        if !object_binding_can_define_property(binding, &property) {
            return;
        }
        object_binding_set_property(binding, property, value);
        let synced_binding = binding.clone();
        self.state
            .speculation
            .static_semantics
            .set_local_object_binding(&target_name, synced_binding.clone());
        if self.binding_name_is_global(&target_name) {
            self.backend
                .sync_global_object_binding(&target_name, Some(synced_binding));
        }
    }

    fn sync_static_class_prototype_init_tracking_effect(
        &mut self,
        target: &Expression,
        prototype_parent: &Expression,
    ) {
        let target = self
            .resolve_bound_alias_expression(target)
            .filter(|resolved| !static_expression_matches(resolved, target))
            .unwrap_or_else(|| self.materialize_static_expression(target));
        let Expression::Identifier(target_name) = target else {
            return;
        };

        let prototype_parent = self
            .resolve_bound_alias_expression(prototype_parent)
            .filter(|resolved| !static_expression_matches(resolved, prototype_parent))
            .unwrap_or_else(|| prototype_parent.clone());
        let prototype_parent =
            self.resolve_static_class_init_local_aliases_in_expression(&prototype_parent);
        let prototype_parent = match prototype_parent {
            Expression::Sequence(expressions) => {
                expressions.last().cloned().unwrap_or(Expression::Undefined)
            }
            other => other,
        };
        let prototype_object = Expression::Call {
            callee: Box::new(Expression::Member {
                object: Box::new(Expression::Identifier("Object".to_string())),
                property: Box::new(Expression::String("create".to_string())),
            }),
            arguments: vec![CallArgument::Expression(prototype_parent.clone())],
        };

        self.update_prototype_object_binding(&target_name, &prototype_object);
        self.backend.sync_global_object_prototype_expression(
            &format!("{target_name}.prototype"),
            Some(prototype_parent),
        );
    }

    pub(in crate::backend::direct_wasm) fn sync_static_statement_tracking_effects(
        &mut self,
        statement: &Statement,
    ) {
        let class_field_initializer_eval_rules =
            self.statement_uses_class_field_initializer_eval_rules(statement);
        self.with_class_field_initializer_eval_scope(
            class_field_initializer_eval_rules,
            |compiler| {
                match statement {
                    Statement::Declaration { body }
                    | Statement::Block { body }
                    | Statement::Labeled { body, .. } => {
                        for statement in body {
                            compiler.sync_static_statement_tracking_effects(statement);
                        }
                    }
                    Statement::If {
                        then_branch,
                        else_branch,
                        ..
                    } => {
                        for statement in then_branch {
                            compiler.sync_static_statement_tracking_effects(statement);
                        }
                        for statement in else_branch {
                            compiler.sync_static_statement_tracking_effects(statement);
                        }
                    }
                    Statement::Var { name, value } | Statement::Let { name, value, .. } => {
                        compiler
                            .sync_static_binding_tracking_effect(name, value)
                            .expect("static statement binding sync should succeed");
                    }
                    Statement::Assign { name, value } => {
                        if compiler.backend.lexical_global_binding(name).is_some() {
                            compiler.clear_global_binding_state(name);
                        } else {
                            compiler
                                .sync_static_binding_tracking_effect(name, value)
                                .expect("static statement binding sync should succeed");
                        }
                    }
                    Statement::Expression(Expression::Call { callee, arguments })
                        if matches!(callee.as_ref(), Expression::Identifier(name) if name == "__ayyClassPrototypeInit") =>
                    {
                        if let [
                            CallArgument::Expression(target),
                            CallArgument::Expression(prototype_parent),
                            ..,
                        ] = arguments.as_slice()
                        {
                            compiler.sync_static_class_prototype_init_tracking_effect(
                                target,
                                prototype_parent,
                            );
                        }
                    }
                    Statement::Expression(Expression::Call { callee, arguments })
                        if matches!(
                            callee.as_ref(),
                            Expression::Member { object, property }
                                if matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
                                    && matches!(property.as_ref(), Expression::String(name) if name == "defineProperty")
                        ) =>
                    {
                        if let [
                            CallArgument::Expression(target),
                            CallArgument::Expression(property),
                            CallArgument::Expression(descriptor),
                            ..,
                        ] = arguments.as_slice()
                        {
                            compiler.sync_static_define_property_tracking_effect(
                                target, property, descriptor,
                            );
                        }
                        let expression = Expression::Call {
                            callee: callee.clone(),
                            arguments: arguments.clone(),
                        };
                        compiler.update_member_function_binding_from_expression(&expression);
                        compiler.update_object_binding_from_expression(&expression);
                    }
                    Statement::Expression(expression) => {
                        compiler.update_member_function_binding_from_expression(expression);
                        compiler.update_object_binding_from_expression(expression);
                    }
                    Statement::AssignMember {
                        object,
                        property,
                        value,
                    } => {
                        compiler.sync_static_assign_member_tracking_effect(object, property, value);
                    }
                    _ => {}
                }
                Ok(())
            },
        )
        .expect("static statement tracking sync should not fail");
    }
}
