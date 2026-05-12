use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn resolve_derived_constructor_super_call(
        &self,
        user_function: &UserFunction,
    ) -> Option<(&Expression, &[CallArgument])> {
        let function = self.resolve_registered_function_declaration(&user_function.name)?;
        let resolved = function.body.iter().find_map(|statement| match statement {
            Statement::Expression(Expression::SuperCall { callee, arguments })
            | Statement::Var {
                value: Expression::SuperCall { callee, arguments },
                ..
            }
            | Statement::Let {
                value: Expression::SuperCall { callee, arguments },
                ..
            }
            | Statement::Assign {
                value: Expression::SuperCall { callee, arguments },
                ..
            }
            | Statement::Return(Expression::SuperCall { callee, arguments }) => {
                Some((callee.as_ref(), arguments.as_slice()))
            }
            _ => None,
        });
        if std::env::var_os("AYY_TRACE_PROXY_DEFINE_PROPERTY").is_some() {
            eprintln!(
                "derived_super_call:function={} found={} body={:?}",
                user_function.name,
                resolved.is_some(),
                function.body
            );
        }
        resolved
    }

    fn resolve_derived_constructor_super_call_replacement_this_expression(
        &self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
    ) -> Option<Expression> {
        let (super_callee, super_arguments) =
            self.resolve_derived_constructor_super_call(user_function)?;
        let this_binding = Expression::Identifier(Self::STATIC_NEW_THIS_BINDING.to_string());
        let arguments_binding = Expression::Identifier("arguments".to_string());
        let substituted_callee = self.substitute_user_function_call_frame_bindings(
            super_callee,
            user_function,
            arguments,
            &this_binding,
            &arguments_binding,
        );
        let resolved_callee = self
            .resolve_bound_alias_expression(&substituted_callee)
            .or_else(|| match &substituted_callee {
                Expression::Identifier(name) => self
                    .state
                    .speculation
                    .static_semantics
                    .local_value_binding(name)
                    .cloned()
                    .or_else(|| {
                        self.backend
                            .global_semantics
                            .values
                            .value_bindings
                            .get(name)
                            .cloned()
                    }),
                _ => None,
            })
            .unwrap_or_else(|| substituted_callee.clone());
        let substituted_arguments = super_arguments
            .iter()
            .map(|argument| match argument {
                CallArgument::Expression(expression) => Some(CallArgument::Expression(
                    self.substitute_user_function_call_frame_bindings(
                        expression,
                        user_function,
                        arguments,
                        &this_binding,
                        &arguments_binding,
                    ),
                )),
                CallArgument::Spread(expression) => Some(CallArgument::Spread(
                    self.substitute_user_function_call_frame_bindings(
                        expression,
                        user_function,
                        arguments,
                        &this_binding,
                        &arguments_binding,
                    ),
                )),
            })
            .collect::<Option<Vec<_>>>()?;
        let LocalFunctionBinding::User(super_function_name) =
            self.resolve_function_binding_from_expression(&resolved_callee)?
        else {
            return None;
        };
        let super_function = self.user_function(&super_function_name)?;
        let capture_source_bindings = self
            .resolve_constructor_capture_source_bindings_from_expression(&Expression::New {
                callee: Box::new(resolved_callee.clone()),
                arguments: substituted_arguments.clone(),
            });
        let replacement = self
            .resolve_registered_function_declaration(&super_function.name)
            .and_then(|function| function.body.last())
            .and_then(|statement| match statement {
                Statement::Return(expression) => Some(expression),
                _ => None,
            })
            .map(|return_value| {
                self.substitute_user_function_call_frame_bindings(
                    return_value,
                    super_function,
                    &substituted_arguments,
                    &Expression::Identifier(Self::STATIC_NEW_THIS_BINDING.to_string()),
                    &Expression::Identifier("arguments".to_string()),
                )
            })
            .or_else(|| {
                self.resolve_user_constructor_return_expression_for_function(
                    super_function,
                    &substituted_arguments,
                    capture_source_bindings.as_ref(),
                )
            });
        if std::env::var_os("AYY_TRACE_PROXY_DEFINE_PROPERTY").is_some() {
            eprintln!(
                "derived_super_replacement:function={} callee={substituted_callee:?} resolved_callee={resolved_callee:?} replacement={replacement:?}",
                user_function.name,
            );
        }
        replacement
    }

    fn resolve_derived_constructor_replacement_this_expression(
        &self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
    ) -> Option<Expression> {
        if let Some(return_value) = self
            .resolve_registered_function_declaration(&user_function.name)
            .and_then(|function| function.body.last())
            .and_then(|statement| match statement {
                Statement::Return(expression) => Some(expression),
                _ => None,
            })
        {
            return Some(self.substitute_user_function_call_frame_bindings(
                return_value,
                user_function,
                arguments,
                &Expression::Identifier(Self::STATIC_NEW_THIS_BINDING.to_string()),
                &Expression::Identifier("arguments".to_string()),
            ));
        }
        let capture_source_bindings = self
            .resolve_constructor_capture_source_bindings_from_expression(&Expression::Identifier(
                user_function.name.clone(),
            ));
        self.resolve_user_constructor_return_expression_for_function(
            user_function,
            arguments,
            capture_source_bindings.as_ref(),
        )
        .or_else(|| {
            self.resolve_derived_constructor_super_call_replacement_this_expression(
                user_function,
                arguments,
            )
        })
    }

    fn sync_runtime_this_shadow_from_expression(
        &mut self,
        this_expression: &Expression,
    ) -> DirectResult<()> {
        if let Some(source_owner) =
            self.resolve_user_function_call_receiver_shadow_owner(this_expression)
            && source_owner != "this"
        {
            if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                eprintln!(
                    "derived_this_runtime_shadow_seed source_owner={source_owner} this_expression={this_expression:?}",
                );
            }
            self.emit_runtime_object_property_shadow_copy(&source_owner, "this")?;
            return Ok(());
        }

        if let Some(object_binding) = self.resolve_object_binding_from_expression(this_expression) {
            if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                eprintln!(
                    "derived_this_runtime_shadow_seed_binding this_expression={this_expression:?}",
                );
            }
            self.emit_runtime_object_property_shadow_seed_from_binding("this", &object_binding)?;
        }

        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn sync_current_derived_constructor_runtime_this_shadow_to_static_owner(
        &mut self,
    ) -> DirectResult<()> {
        let Some(this_expression) = self
            .state
            .speculation
            .static_semantics
            .local_value_binding("this")
            .cloned()
        else {
            return Ok(());
        };
        let Some(target_owner) =
            self.resolve_user_function_call_receiver_shadow_owner(&this_expression)
        else {
            return Ok(());
        };
        if target_owner == "this" {
            return Ok(());
        }
        if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
            eprintln!(
                "derived_this_runtime_shadow_commit target_owner={target_owner} this_expression={this_expression:?}",
            );
        }
        self.emit_runtime_object_property_shadow_copy("this", &target_owner)
    }

    pub(super) fn sync_derived_constructor_this_binding_after_super_call(
        &mut self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        super_target_name: &str,
    ) -> DirectResult<()> {
        let this_expression = self
            .resolve_derived_constructor_replacement_this_expression(user_function, arguments)
            .unwrap_or_else(|| Expression::New {
                callee: Box::new(Expression::Identifier(user_function.name.clone())),
                arguments: arguments.to_vec(),
            });
        let trace_proxy_define_property =
            std::env::var_os("AYY_TRACE_PROXY_DEFINE_PROPERTY").is_some();
        if trace_proxy_define_property {
            eprintln!(
                "derived_this_sync:function={} this_expression={this_expression:?}",
                user_function.name
            );
        }
        self.state
            .speculation
            .static_semantics
            .set_local_value_binding("this", this_expression.clone());
        self.state
            .speculation
            .static_semantics
            .set_local_kind("this", StaticValueKind::Object);
        if let Some(proxy_binding) = self.resolve_proxy_binding_from_expression(&this_expression) {
            if trace_proxy_define_property {
                eprintln!(
                    "derived_this_sync:function={} proxy_target={:?}",
                    user_function.name, proxy_binding.target
                );
            }
            self.state
                .speculation
                .static_semantics
                .set_local_proxy_binding("this", proxy_binding);
        } else {
            if trace_proxy_define_property {
                eprintln!(
                    "derived_this_sync:function={} proxy_target=<none>",
                    user_function.name
                );
            }
            self.state
                .speculation
                .static_semantics
                .clear_local_proxy_binding("this");
        }
        let capture_source_bindings = self
            .resolve_constructor_capture_source_bindings_from_expression(&Expression::New {
                callee: Box::new(Expression::Identifier(user_function.name.clone())),
                arguments: arguments.to_vec(),
            });
        let this_binding = self
            .resolve_user_constructor_object_binding_from_new(
                &Expression::Identifier(user_function.name.clone()),
                arguments,
            )
            .or_else(|| {
                self.resolve_user_constructor_object_binding_for_function(
                    user_function,
                    arguments,
                    capture_source_bindings.as_ref(),
                )
            });
        if let Some(this_binding) = this_binding {
            self.state
                .speculation
                .static_semantics
                .set_local_object_binding("this", this_binding.clone());
            self.state
                .speculation
                .static_semantics
                .set_local_value_binding(super_target_name, Expression::Object(Vec::new()));
            self.state
                .speculation
                .static_semantics
                .set_local_object_binding(super_target_name, this_binding);
            self.state
                .speculation
                .static_semantics
                .set_local_kind(super_target_name, StaticValueKind::Object);
        }
        self.sync_runtime_this_shadow_from_expression(&this_expression)
    }

    pub(super) fn sync_derived_constructor_this_binding_after_builtin_super_call(&mut self) {
        self.state
            .speculation
            .static_semantics
            .set_local_value_binding("this", Expression::Object(Vec::new()));
        self.state
            .speculation
            .static_semantics
            .set_local_kind("this", StaticValueKind::Object);
        self.state
            .speculation
            .static_semantics
            .clear_local_object_binding("this");
        self.state
            .speculation
            .static_semantics
            .clear_local_proxy_binding("this");
    }
}
