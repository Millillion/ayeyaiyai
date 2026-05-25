use super::*;

impl<'a> FunctionCompiler<'a> {
    fn resolve_test262_realm_global_property_value(
        &self,
        realm_id: u32,
        property_name: &str,
    ) -> Option<Expression> {
        let property = Expression::String(property_name.to_string());
        self.test262_realm_global_object_binding(realm_id)
            .and_then(|binding| object_binding_lookup_value(&binding, &property).cloned())
    }

    fn resolve_test262_realm_constructor_prototype_binding(
        &self,
        realm_id: u32,
        constructor_name: &str,
    ) -> Option<ObjectValueBinding> {
        let constructor =
            self.resolve_test262_realm_global_property_value(realm_id, constructor_name)?;
        let constructor_binding = self.resolve_object_binding_from_expression(&constructor)?;
        let prototype = object_binding_lookup_value(
            &constructor_binding,
            &Expression::String("prototype".to_string()),
        )?;
        self.resolve_object_binding_from_expression(prototype)
    }

    fn test262_realm_primitive_constructor_name(&self, value: &Expression) -> Option<&'static str> {
        match value {
            Expression::Number(_) => Some("Number"),
            Expression::String(_) => Some("String"),
            Expression::Bool(_) => Some("Boolean"),
            Expression::BigInt(_) => Some("BigInt"),
            Expression::Call { callee, .. } if matches!(callee.as_ref(), Expression::Identifier(name) if name == "Symbol") => {
                Some("Symbol")
            }
            Expression::Identifier(name)
                if self.lookup_identifier_kind(name) == Some(StaticValueKind::Symbol) =>
            {
                Some("Symbol")
            }
            _ => None,
        }
    }

    fn resolve_test262_realm_eval_member_expression(
        &self,
        realm_id: u32,
        object: &Expression,
        property: &Expression,
    ) -> Option<Expression> {
        let property = self.canonical_object_property_expression(property);
        let object_value = self
            .resolve_test262_realm_eval_expression(realm_id, object)
            .unwrap_or_else(|| object.clone());

        if let Some(object_binding) = self.resolve_object_binding_from_expression(&object_value)
            && let Some(value) =
                self.resolve_object_binding_property_value(&object_binding, &property)
        {
            return Some(value);
        }

        let constructor_name = self.test262_realm_primitive_constructor_name(&object_value)?;
        let prototype_binding =
            self.resolve_test262_realm_constructor_prototype_binding(realm_id, constructor_name)?;
        self.resolve_object_binding_property_value(&prototype_binding, &property)
    }

    fn resolve_test262_realm_eval_expression(
        &self,
        realm_id: u32,
        expression: &Expression,
    ) -> Option<Expression> {
        match expression {
            Expression::Identifier(name) => self
                .resolve_test262_realm_global_property_value(realm_id, name)
                .or_else(|| matches!(name.as_str(), "undefined").then_some(Expression::Undefined)),
            Expression::Member { object, property } => {
                self.resolve_test262_realm_eval_member_expression(realm_id, object, property)
            }
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::Object(_)
            | Expression::Array(_) => Some(expression.clone()),
            Expression::Call { callee, .. } if matches!(callee.as_ref(), Expression::Identifier(name) if name == "Symbol") => {
                Some(expression.clone())
            }
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn emit_function_constructor_builtin_call(
        &mut self,
        name: &str,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        if !is_function_constructor_builtin(name) {
            return Ok(false);
        }

        if let Some((parameter_source, body_source)) =
            function_constructor_literal_source_parts(arguments)
        {
            let wrappers =
                function_constructor_wrapper_sources(name, &parameter_source, &body_source)
                    .expect("checked builtin names should produce wrapper sources");
            let parses = wrappers
                .iter()
                .any(|wrapper| frontend::parse(wrapper).is_ok());
            if !parses {
                self.emit_named_error_throw("SyntaxError")?;
                return Ok(true);
            }
        }

        for argument in arguments {
            match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    self.emit_numeric_expression(expression)?;
                    self.state.emission.output.instructions.push(0x1a);
                }
            }
        }
        self.push_i32_const(JS_TYPEOF_FUNCTION_TAG);
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_test262_realm_eval_call(
        &mut self,
        builtin_name: &str,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let Some(realm_id) = parse_test262_realm_eval_builtin(builtin_name) else {
            return Ok(false);
        };
        let Some(argument) = arguments.first() else {
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(true);
        };

        let CallArgument::Expression(Expression::String(argument_source)) = argument else {
            return Ok(false);
        };
        let Ok(program) = frontend::parse_script_goal(argument_source) else {
            self.emit_named_error_throw("SyntaxError")?;
            return Ok(true);
        };

        let realm_member_assignment = match program.statements.as_slice() {
            [
                Statement::AssignMember {
                    object,
                    property,
                    value,
                },
            ] => Some((object, property, value)),
            [
                Statement::Expression(Expression::AssignMember {
                    object,
                    property,
                    value,
                }),
            ] => Some((object.as_ref(), property.as_ref(), value.as_ref())),
            _ => None,
        };
        if let Some((object, property, value)) = realm_member_assignment {
            for argument in arguments.iter().skip(1) {
                match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        self.emit_numeric_expression(expression)?;
                        self.state.emission.output.instructions.push(0x1a);
                    }
                }
            }
            if self.emit_primitive_prototype_proxy_set_assignment(
                object,
                property,
                value,
                Some(realm_id),
            )? {
                return Ok(true);
            }
        }

        if let [Statement::Expression(expression)] = program.statements.as_slice()
            && let Some(value) = self.resolve_test262_realm_eval_expression(realm_id, expression)
        {
            for argument in arguments.iter().skip(1) {
                match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        self.emit_numeric_expression(expression)?;
                        self.state.emission.output.instructions.push(0x1a);
                    }
                }
            }
            self.emit_numeric_expression(&value)?;
            return Ok(true);
        }

        let [Statement::Var { name, value }] = program.statements.as_slice() else {
            return self.emit_indirect_eval_call_with_context(arguments, Some(builtin_name));
        };
        let materialized_value = self.materialize_static_expression(value);
        let Some(realm) = self.test262_realm_mut(realm_id) else {
            return Ok(false);
        };
        object_binding_set_property(
            &mut realm.global_object_binding,
            Expression::String(name.clone()),
            materialized_value,
        );

        for argument in arguments.iter().skip(1) {
            match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    self.emit_numeric_expression(expression)?;
                    self.state.emission.output.instructions.push(0x1a);
                }
            }
        }
        self.push_i32_const(JS_UNDEFINED_TAG);
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_is_nan_call(
        &mut self,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let first_argument = arguments.first();

        if let Some(CallArgument::Expression(Expression::String(text))) = first_argument {
            for argument in arguments.iter().skip(1) {
                match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        self.emit_numeric_expression(expression)?;
                        self.state.emission.output.instructions.push(0x1a);
                    }
                }
            }
            self.push_i32_const(if parse_string_to_i32(text).is_ok() {
                0
            } else {
                1
            });
            return Ok(true);
        }

        if matches!(
            first_argument,
            Some(CallArgument::Expression(
                Expression::Object(_) | Expression::Array(_) | Expression::This
            ))
        ) {
            for argument in arguments.iter() {
                match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        self.emit_numeric_expression(expression)?;
                        self.state.emission.output.instructions.push(0x1a);
                    }
                }
            }
            self.push_i32_const(1);
            return Ok(true);
        }

        let value_local = self.allocate_temp_local();
        match first_argument {
            Some(CallArgument::Expression(expression) | CallArgument::Spread(expression)) => {
                self.emit_numeric_expression(expression)?;
            }
            None => self.push_i32_const(JS_UNDEFINED_TAG),
        }
        self.push_local_set(value_local);

        for argument in arguments.iter().skip(1) {
            match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    self.emit_numeric_expression(expression)?;
                    self.state.emission.output.instructions.push(0x1a);
                }
            }
        }

        self.push_local_get(value_local);
        self.push_i32_const(JS_NAN_TAG);
        self.push_binary_op(BinaryOp::Equal)?;
        self.push_local_get(value_local);
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_binary_op(BinaryOp::Equal)?;
        self.push_binary_op(BinaryOp::BitwiseOr)?;
        Ok(true)
    }
}
