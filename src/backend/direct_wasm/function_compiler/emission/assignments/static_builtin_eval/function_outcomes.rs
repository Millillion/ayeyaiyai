use super::*;

impl<'a> FunctionCompiler<'a> {
    fn user_function_private_member_reads_resolve_to_data_values(
        &self,
        user_function: &UserFunction,
        this_binding: &Expression,
        this_object_binding: &ObjectValueBinding,
    ) -> bool {
        struct PrivatePropertyCollector {
            names: Vec<String>,
        }
        impl crate::ir::visit::Visitor for PrivatePropertyCollector {
            fn visit_expression(&mut self, expression: &Expression) {
                match expression {
                    Expression::Member { property, .. }
                    | Expression::AssignMember { property, .. } => {
                        if let Expression::String(name) = property.as_ref()
                            && name.starts_with("__ayy$private$")
                        {
                            self.names.push(name.clone());
                        }
                    }
                    _ => {}
                }
                crate::ir::visit::walk_expression(self, expression);
            }
            fn visit_statement(&mut self, statement: &Statement) {
                if let Statement::AssignMember { property, .. } = statement
                    && let Expression::String(name) = property
                    && name.starts_with("__ayy$private$")
                {
                    self.names.push(name.clone());
                }
                crate::ir::visit::walk_statement(self, statement);
            }
        }
        let Some(function) = self.resolve_registered_function_declaration(&user_function.name)
        else {
            return false;
        };
        let mut collector = PrivatePropertyCollector { names: Vec::new() };
        for statement in &function.body {
            crate::ir::visit::Visitor::visit_statement(&mut collector, statement);
        }
        collector.names.iter().all(|name| {
            let property = Expression::String(name.clone());
            object_binding_lookup_value(this_object_binding, &property).is_some()
                && self
                    .resolve_member_getter_binding(this_binding, &property)
                    .is_none()
                && self
                    .resolve_member_setter_binding(this_binding, &property)
                    .is_none()
        })
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_function_outcome_from_binding_with_call_frame_and_context(
        &self,
        binding: &LocalFunctionBinding,
        arguments: &[CallArgument],
        this_binding: &Expression,
        current_function_name: Option<&str>,
    ) -> Option<StaticEvalOutcome> {
        let LocalFunctionBinding::User(function_name) = binding else {
            return self.resolve_static_function_outcome_from_binding_with_context(
                binding,
                arguments,
                current_function_name,
            );
        };
        let user_function = self.user_function(function_name)?;
        if self.user_function_mentions_private_member_access(user_function) {
            let Some(this_object_binding) =
                self.resolve_object_binding_from_expression(this_binding)
            else {
                return None;
            };
            // Folding is only sound when every private member the body reads
            // resolves to a plain data value on the receiver (instance
            // fields). Private accessors and methods are modeled as member
            // bindings/descriptors, so substituting the raw property value
            // would surface the accessor function instead of dispatching it.
            if !self.user_function_private_member_reads_resolve_to_data_values(
                user_function,
                this_binding,
                &this_object_binding,
            ) {
                return None;
            }
        }
        let function = self.resolve_registered_function_declaration(function_name)?;
        if self.user_function_mentions_direct_eval(user_function) {
            return self.resolve_static_direct_eval_return_outcome_from_user_function(
                user_function,
                function,
                arguments,
                this_binding,
            );
        }
        if function.body.is_empty() {
            return Some(StaticEvalOutcome::Value(Expression::Undefined));
        }
        if user_function.has_parameter_defaults() {
            let expanded_arguments = self.expand_call_arguments(arguments);
            return self
                .resolve_bound_snapshot_user_function_outcome_with_arguments_and_this(
                    function_name,
                    &HashMap::new(),
                    &expanded_arguments,
                    this_binding,
                )
                .map(|(outcome, _)| outcome);
        }
        let [statement] = function.body.as_slice() else {
            return None;
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
        match statement {
            Statement::Return(expression) => {
                let value = self.substitute_user_function_call_frame_bindings(
                    expression,
                    user_function,
                    arguments,
                    this_binding,
                    &arguments_binding,
                );
                Some(StaticEvalOutcome::Value(
                    self.resolve_static_super_members_in_call_frame_return(
                        &value,
                        function_name,
                        this_binding,
                    ),
                ))
            }
            Statement::Throw(expression) => {
                let value = self.substitute_user_function_call_frame_bindings(
                    expression,
                    user_function,
                    arguments,
                    this_binding,
                    &arguments_binding,
                );
                Some(StaticEvalOutcome::Throw(StaticThrowValue::Value(
                    self.resolve_static_super_members_in_call_frame_return(
                        &value,
                        function_name,
                        this_binding,
                    ),
                )))
            }
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_function_outcome_from_binding_with_context(
        &self,
        binding: &LocalFunctionBinding,
        arguments: &[CallArgument],
        current_function_name: Option<&str>,
    ) -> Option<StaticEvalOutcome> {
        let LocalFunctionBinding::User(function_name) = binding else {
            let LocalFunctionBinding::Builtin(function_name) = binding else {
                return None;
            };
            return self.resolve_static_builtin_function_outcome(
                function_name,
                arguments,
                current_function_name,
            );
        };
        let user_function = self.user_function(function_name)?;
        if self.user_function_mentions_private_member_access(user_function) {
            return None;
        }

        let function = self.resolve_registered_function_declaration(function_name)?;
        if self.user_function_mentions_direct_eval(user_function) {
            let this_binding =
                if self.should_box_sloppy_function_this(user_function, &Expression::Undefined) {
                    Expression::Identifier("globalThis".to_string())
                } else {
                    Expression::Undefined
                };
            return self.resolve_static_direct_eval_return_outcome_from_user_function(
                user_function,
                function,
                arguments,
                &this_binding,
            );
        }
        if function.body.is_empty() {
            return Some(StaticEvalOutcome::Value(Expression::Undefined));
        }
        let this_binding =
            if self.should_box_sloppy_function_this(user_function, &Expression::Undefined) {
                Expression::Identifier("globalThis".to_string())
            } else {
                Expression::Undefined
            };
        if user_function.has_parameter_defaults() {
            let expanded_arguments = self.expand_call_arguments(arguments);
            return self
                .resolve_bound_snapshot_user_function_outcome_with_arguments_and_this(
                    function_name,
                    &HashMap::new(),
                    &expanded_arguments,
                    &this_binding,
                )
                .map(|(outcome, _)| outcome);
        }
        let [statement] = function.body.as_slice() else {
            return None;
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
        match statement {
            Statement::Return(expression) => Some(StaticEvalOutcome::Value(
                self.substitute_user_function_call_frame_bindings(
                    expression,
                    user_function,
                    arguments,
                    &this_binding,
                    &arguments_binding,
                ),
            )),
            Statement::Throw(expression) => Some(StaticEvalOutcome::Throw(
                StaticThrowValue::Value(self.substitute_user_function_call_frame_bindings(
                    expression,
                    user_function,
                    arguments,
                    &this_binding,
                    &arguments_binding,
                )),
            )),
            _ => None,
        }
    }
}
