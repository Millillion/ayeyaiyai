use super::*;

impl<'a> FunctionCompiler<'a> {
    fn collect_direct_arguments_assignment_targets_from_expression(
        expression: &Expression,
        targets: &mut Vec<String>,
    ) {
        match expression {
            Expression::Assign { name, value } if Self::is_direct_arguments_identifier(value) => {
                if !targets.contains(name) {
                    targets.push(name.clone());
                }
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => Self::collect_direct_arguments_assignment_targets_from_expression(value, targets),
            Expression::Member { object, property }
            | Expression::AssignMember {
                object,
                property,
                value: _,
            } => {
                Self::collect_direct_arguments_assignment_targets_from_expression(object, targets);
                Self::collect_direct_arguments_assignment_targets_from_expression(
                    property, targets,
                );
                if let Expression::AssignMember { value, .. } = expression {
                    Self::collect_direct_arguments_assignment_targets_from_expression(
                        value, targets,
                    );
                }
            }
            Expression::SuperMember { property } => {
                Self::collect_direct_arguments_assignment_targets_from_expression(
                    property, targets,
                );
            }
            Expression::AssignSuperMember { property, value } => {
                Self::collect_direct_arguments_assignment_targets_from_expression(
                    property, targets,
                );
                Self::collect_direct_arguments_assignment_targets_from_expression(value, targets);
            }
            Expression::Binary { left, right, .. } => {
                Self::collect_direct_arguments_assignment_targets_from_expression(left, targets);
                Self::collect_direct_arguments_assignment_targets_from_expression(right, targets);
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::collect_direct_arguments_assignment_targets_from_expression(
                    condition, targets,
                );
                Self::collect_direct_arguments_assignment_targets_from_expression(
                    then_expression,
                    targets,
                );
                Self::collect_direct_arguments_assignment_targets_from_expression(
                    else_expression,
                    targets,
                );
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    Self::collect_direct_arguments_assignment_targets_from_expression(
                        expression, targets,
                    );
                }
            }
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                Self::collect_direct_arguments_assignment_targets_from_expression(callee, targets);
                for argument in arguments {
                    Self::collect_direct_arguments_assignment_targets_from_expression(
                        argument.expression(),
                        targets,
                    );
                }
            }
            Expression::Array(elements) => {
                for element in elements {
                    match element {
                        ArrayElement::Expression(value) | ArrayElement::Spread(value) => {
                            Self::collect_direct_arguments_assignment_targets_from_expression(
                                value, targets,
                            );
                        }
                    }
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            Self::collect_direct_arguments_assignment_targets_from_expression(
                                key, targets,
                            );
                            Self::collect_direct_arguments_assignment_targets_from_expression(
                                value, targets,
                            );
                        }
                        ObjectEntry::Getter { key, getter } => {
                            Self::collect_direct_arguments_assignment_targets_from_expression(
                                key, targets,
                            );
                            Self::collect_direct_arguments_assignment_targets_from_expression(
                                getter, targets,
                            );
                        }
                        ObjectEntry::Setter { key, setter } => {
                            Self::collect_direct_arguments_assignment_targets_from_expression(
                                key, targets,
                            );
                            Self::collect_direct_arguments_assignment_targets_from_expression(
                                setter, targets,
                            );
                        }
                        ObjectEntry::Spread(value) => {
                            Self::collect_direct_arguments_assignment_targets_from_expression(
                                value, targets,
                            );
                        }
                    }
                }
            }
            Expression::Update { .. }
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::NewTarget
            | Expression::Identifier(_)
            | Expression::This
            | Expression::Sent => {}
        }
    }

    fn collect_direct_arguments_assignment_targets_from_statement(
        statement: &Statement,
        targets: &mut Vec<String>,
    ) {
        match statement {
            Statement::Assign {
                name,
                value: Expression::Identifier(value_name),
            }
            | Statement::Var {
                name,
                value: Expression::Identifier(value_name),
            }
            | Statement::Let {
                name,
                value: Expression::Identifier(value_name),
                ..
            } if value_name == "arguments" => {
                if !targets.contains(name) {
                    targets.push(name.clone());
                }
            }
            Statement::Expression(Expression::Assign { name, value }) if matches!(value.as_ref(), Expression::Identifier(value_name) if value_name == "arguments") => {
                if !targets.contains(name) {
                    targets.push(name.clone());
                }
            }
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. }
            | Statement::While { body, .. }
            | Statement::DoWhile { body, .. } => {
                for statement in body {
                    Self::collect_direct_arguments_assignment_targets_from_statement(
                        statement, targets,
                    );
                }
            }
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => {
                for statement in then_branch.iter().chain(else_branch) {
                    Self::collect_direct_arguments_assignment_targets_from_statement(
                        statement, targets,
                    );
                }
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                for statement in body.iter().chain(catch_setup).chain(catch_body) {
                    Self::collect_direct_arguments_assignment_targets_from_statement(
                        statement, targets,
                    );
                }
            }
            Statement::Switch { cases, .. } => {
                for case in cases {
                    for statement in &case.body {
                        Self::collect_direct_arguments_assignment_targets_from_statement(
                            statement, targets,
                        );
                    }
                }
            }
            Statement::For { init, body, .. } => {
                for statement in init.iter().chain(body) {
                    Self::collect_direct_arguments_assignment_targets_from_statement(
                        statement, targets,
                    );
                }
            }
            _ => {}
        }
    }

    fn is_direct_arguments_identifier(expression: &Expression) -> bool {
        matches!(
            expression,
            Expression::Identifier(name)
                if name == "arguments"
                    || scoped_binding_source_name(name)
                        .is_some_and(|source_name| source_name == "arguments")
        )
    }

    pub(in crate::backend::direct_wasm) fn sync_direct_arguments_assignments_from_static_user_call(
        &mut self,
        user_function: &UserFunction,
        arguments: &[Expression],
    ) {
        if user_function.lexical_this
            || user_function.params.iter().any(|param| {
                param == "arguments"
                    || scoped_binding_source_name(param)
                        .is_some_and(|source_name| source_name == "arguments")
            })
        {
            return;
        }
        let Some(declaration) = self.resolve_registered_function_declaration(&user_function.name)
        else {
            return;
        };
        let mut targets = Vec::new();
        for default in user_function.parameter_defaults.iter().flatten() {
            Self::collect_direct_arguments_assignment_targets_from_expression(
                default,
                &mut targets,
            );
        }
        for statement in &declaration.body {
            Self::collect_direct_arguments_assignment_targets_from_statement(
                statement,
                &mut targets,
            );
        }
        if targets.is_empty() {
            return;
        }
        let arguments_binding =
            ArgumentsValueBinding::for_user_function(user_function, arguments.to_vec());
        for target in targets {
            if user_function.scope_bindings.contains(&target) {
                continue;
            }
            self.backend
                .sync_global_arguments_binding(&target, Some(arguments_binding.clone()));
        }
    }

    pub(in crate::backend::direct_wasm) fn emit_builtin_call(
        &mut self,
        name: &str,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let object_identifier = Expression::Identifier("Object".to_string());
        let array_identifier = Expression::Identifier("Array".to_string());
        let reflect_identifier = Expression::Identifier("Reflect".to_string());
        if let Some(target_name) = parse_bound_function_prototype_call_builtin_name(name) {
            return self.emit_bound_function_prototype_call_builtin(target_name, arguments);
        }

        if matches!(
            name,
            "__assert" | "__assertSameValue" | "__assertNotSameValue"
        ) {
            return self.emit_assertion_builtin_call(name, arguments);
        }

        if name == "isNaN" {
            return self.emit_is_nan_call(arguments);
        }

        if name == "eval" {
            return self.emit_eval_call(arguments);
        }

        if name == TEST262_CREATE_REALM_BUILTIN {
            for argument in arguments {
                match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        self.emit_numeric_expression(expression)?;
                        self.state.emission.output.instructions.push(0x1a);
                    }
                }
            }
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            return Ok(true);
        }

        if self.emit_test262_realm_eval_call(name, arguments)? {
            return Ok(true);
        }

        if self.emit_function_constructor_builtin_call(name, arguments)? {
            return Ok(true);
        }

        if name == "String" {
            for argument in arguments {
                match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        self.emit_numeric_expression(expression)?;
                        self.state.emission.output.instructions.push(0x1a);
                    }
                }
            }
            let Some(value) = self.resolve_static_builtin_primitive_call_value(
                name,
                arguments,
                self.current_function_name(),
            ) else {
                self.push_i32_const(JS_TYPEOF_STRING_TAG);
                return Ok(true);
            };
            self.emit_numeric_expression(&value)?;
            return Ok(true);
        }

        if name == "JSON.stringify" {
            for argument in arguments {
                match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        self.emit_numeric_expression(expression)?;
                        self.state.emission.output.instructions.push(0x1a);
                    }
                }
            }
            if let Some(value) = self.resolve_static_builtin_primitive_call_value(
                name,
                arguments,
                self.current_function_name(),
            ) {
                self.emit_numeric_expression(&value)?;
            } else {
                self.push_i32_const(JS_TYPEOF_STRING_TAG);
            }
            return Ok(true);
        }

        if name == "Math.floor" {
            let value_local = self.allocate_temp_local();
            match arguments.first() {
                Some(CallArgument::Expression(expression) | CallArgument::Spread(expression)) => {
                    self.emit_numeric_expression(expression)?;
                }
                None => self.push_i32_const(JS_NAN_TAG),
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
            return Ok(true);
        }

        if matches!(
            name,
            "Math.abs"
                | "Math.atan"
                | "Math.exp"
                | "Math.max"
                | "Math.min"
                | "Math.pow"
                | "Math.sin"
        ) {
            for argument in arguments {
                match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        self.emit_numeric_expression(expression)?;
                        self.state.emission.output.instructions.push(0x1a);
                    }
                }
            }
            self.push_i32_const(0);
            return Ok(true);
        }

        match name {
            "Array.isArray" => {
                return self.emit_array_is_array_call(
                    &array_identifier,
                    &Expression::String("isArray".to_string()),
                    arguments,
                );
            }
            "Object.create" => {
                return self.emit_object_create_call(
                    &object_identifier,
                    &Expression::String("create".to_string()),
                    arguments,
                );
            }
            "Object.getOwnPropertyDescriptor" => {
                return self.emit_object_get_own_property_descriptor_call(
                    &object_identifier,
                    &Expression::String("getOwnPropertyDescriptor".to_string()),
                    arguments,
                );
            }
            "Object.getOwnPropertyNames" => {
                return self.emit_object_array_builtin_call(
                    &object_identifier,
                    &Expression::String("getOwnPropertyNames".to_string()),
                    arguments,
                );
            }
            "Object.getOwnPropertySymbols" => {
                return self.emit_object_array_builtin_call(
                    &object_identifier,
                    &Expression::String("getOwnPropertySymbols".to_string()),
                    arguments,
                );
            }
            "Object.getPrototypeOf" => {
                return self.emit_object_get_prototype_of_call(
                    &object_identifier,
                    &Expression::String("getPrototypeOf".to_string()),
                    arguments,
                );
            }
            "Object.defineProperty" => {
                return self.emit_object_define_property_call(
                    &object_identifier,
                    &Expression::String("defineProperty".to_string()),
                    arguments,
                );
            }
            "Object.is" => {
                return self.emit_object_is_call(
                    &object_identifier,
                    &Expression::String("is".to_string()),
                    arguments,
                );
            }
            "Object.isExtensible" => {
                return self.emit_object_is_extensible_call(
                    &object_identifier,
                    &Expression::String("isExtensible".to_string()),
                    arguments,
                );
            }
            "Object.keys" => {
                return self.emit_object_array_builtin_call(
                    &object_identifier,
                    &Expression::String("keys".to_string()),
                    arguments,
                );
            }
            "Object.preventExtensions" => {
                return self.emit_object_prevent_extensions_call(
                    &object_identifier,
                    &Expression::String("preventExtensions".to_string()),
                    arguments,
                );
            }
            "Object.setPrototypeOf" => {
                return self.emit_object_set_prototype_of_call(
                    &object_identifier,
                    &Expression::String("setPrototypeOf".to_string()),
                    arguments,
                );
            }
            "Reflect.defineProperty" => {
                return self.emit_reflect_define_property_call(
                    &reflect_identifier,
                    &Expression::String("defineProperty".to_string()),
                    arguments,
                );
            }
            "Reflect.has" => {
                return self.emit_reflect_has_call(
                    &reflect_identifier,
                    &Expression::String("has".to_string()),
                    arguments,
                );
            }
            "Reflect.preventExtensions" => {
                return self.emit_object_prevent_extensions_call(
                    &reflect_identifier,
                    &Expression::String("preventExtensions".to_string()),
                    arguments,
                );
            }
            _ => {}
        }

        if let Some(native_error_value) = native_error_runtime_value(name) {
            for argument in arguments {
                match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        self.emit_numeric_expression(expression)?;
                        self.state.emission.output.instructions.push(0x1a);
                    }
                }
            }
            self.push_i32_const(native_error_value);
            return Ok(true);
        }

        if name == "Promise" {
            let expanded_arguments = self.expand_call_arguments(arguments);
            let Some(raw_executor) = expanded_arguments.first() else {
                self.emit_named_error_throw("TypeError")?;
                self.push_i32_const(JS_UNDEFINED_TAG);
                return Ok(true);
            };
            let executor = self
                .resolve_array_binding_from_expression(raw_executor)
                .and_then(|binding| binding.values.first().cloned())
                .flatten()
                .or_else(|| {
                    self.resolve_arguments_binding_from_expression(raw_executor)
                        .and_then(|binding| binding.values.first().cloned())
                })
                .unwrap_or_else(|| raw_executor.clone());
            let materialized_executor = self
                .resolve_bound_alias_expression(&executor)
                .filter(|resolved| !static_expression_matches(resolved, &executor))
                .unwrap_or_else(|| self.materialize_static_expression(&executor));
            let executor_binding = self
                .resolve_function_binding_from_expression(&executor)
                .or_else(|| self.resolve_function_binding_from_expression(&materialized_executor));
            if std::env::var_os("AYY_TRACE_PROMISE_CTOR").is_some() {
                eprintln!(
                    "promise_ctor executor={executor:?} materialized={materialized_executor:?} binding={executor_binding:?} kind={} materialized_kind={}",
                    self.infer_value_kind(&executor)
                        .and_then(StaticValueKind::as_typeof_str)
                        .unwrap_or("unknown"),
                    self.infer_value_kind(&materialized_executor)
                        .and_then(StaticValueKind::as_typeof_str)
                        .unwrap_or("unknown")
                );
            }
            match executor_binding {
                Some(LocalFunctionBinding::User(function_name)) => {
                    let Some(user_function) = self.user_function(&function_name).cloned() else {
                        self.emit_named_error_throw("TypeError")?;
                        self.push_i32_const(JS_UNDEFINED_TAG);
                        return Ok(true);
                    };
                    let callback_arguments = [
                        CallArgument::Expression(Expression::Identifier(
                            TEST262_CREATE_REALM_BUILTIN.to_string(),
                        )),
                        CallArgument::Expression(Expression::Identifier(
                            TEST262_CREATE_REALM_BUILTIN.to_string(),
                        )),
                    ];
                    self.emit_user_function_call(&user_function, &callback_arguments)?;
                    self.sync_direct_arguments_assignments_from_static_user_call(
                        &user_function,
                        &callback_arguments
                            .iter()
                            .map(|argument| match argument {
                                CallArgument::Expression(expression)
                                | CallArgument::Spread(expression) => expression.clone(),
                            })
                            .collect::<Vec<_>>(),
                    );
                    self.state.emission.output.instructions.push(0x1a);
                }
                Some(LocalFunctionBinding::Builtin(_)) => {
                    self.emit_numeric_expression(&executor)?;
                    self.state.emission.output.instructions.push(0x1a);
                }
                None => {
                    let executor_kind = self
                        .infer_value_kind(&executor)
                        .or_else(|| self.infer_value_kind(&materialized_executor));
                    if matches!(
                        executor_kind,
                        None | Some(StaticValueKind::Unknown | StaticValueKind::Function)
                    ) {
                        self.emit_numeric_expression(&executor)?;
                        self.state.emission.output.instructions.push(0x1a);
                    } else {
                        self.emit_numeric_expression(&executor)?;
                        self.state.emission.output.instructions.push(0x1a);
                        self.emit_named_error_throw("TypeError")?;
                        self.push_i32_const(JS_UNDEFINED_TAG);
                        return Ok(true);
                    }
                }
            }
            for argument in expanded_arguments.iter().skip(1) {
                self.emit_numeric_expression(argument)?;
                self.state.emission.output.instructions.push(0x1a);
            }
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            return Ok(true);
        }

        let Some(result_tag) = (match name {
            "Promise.resolve" | "Promise.reject" => Some(JS_TYPEOF_OBJECT_TAG),
            "Number" => Some(JS_TYPEOF_NUMBER_TAG),
            "Boolean" => Some(JS_TYPEOF_BOOLEAN_TAG),
            "Object" | "Array" | "ArrayBuffer" | "SharedArrayBuffer" | "DataView" | "Date"
            | "RegExp" | "Map" | "Set" | "Error" | "EvalError" | "RangeError"
            | "ReferenceError" | "SyntaxError" | "TypeError" | "URIError" | "AggregateError"
            | "SuppressedError" | "Promise" | "WeakMap" | "WeakRef" | "WeakSet" | "Uint8Array"
            | "Int8Array" | "Uint16Array" | "Int16Array" | "Uint32Array" | "Int32Array"
            | "Float32Array" | "Float64Array" | "Uint8ClampedArray" | "BigInt64Array"
            | "BigUint64Array" => Some(JS_TYPEOF_OBJECT_TAG),
            "BigInt" => Some(JS_TYPEOF_BIGINT_TAG),
            "Symbol" => Some(JS_TYPEOF_SYMBOL_TAG),
            _ => None,
        }) else {
            return Ok(false);
        };

        for argument in arguments {
            match argument {
                CallArgument::Expression(expression) => self.emit_numeric_expression(expression)?,
                CallArgument::Spread(expression) => {
                    self.emit_numeric_expression(expression)?;
                }
            }
            self.state.emission.output.instructions.push(0x1a);
        }
        self.push_i32_const(result_tag);
        Ok(true)
    }
}
