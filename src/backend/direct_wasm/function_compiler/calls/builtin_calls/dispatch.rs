use super::*;

impl<'a> FunctionCompiler<'a> {
    fn emit_promise_resolve_user_call_argument_side_effects(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<bool> {
        let Expression::Call { callee, arguments } = expression else {
            return Ok(false);
        };
        if arguments
            .iter()
            .any(|argument| matches!(argument, CallArgument::Spread(_)))
        {
            return Ok(false);
        }
        let Some(LocalFunctionBinding::User(function_name)) =
            self.resolve_function_binding_from_expression(callee)
        else {
            return Ok(false);
        };
        let Some(user_function) = self.user_function(&function_name).cloned() else {
            return Ok(false);
        };
        if user_function.is_async()
            || user_function.is_generator()
            || user_function.has_parameter_defaults()
            || !user_function.extra_argument_indices.is_empty()
            || !self.user_function_has_explicit_call_frame_inlineable_terminal_body(&user_function)
        {
            return Ok(false);
        }
        let expanded_arguments = self.expand_call_arguments(arguments);
        if expanded_arguments
            .iter()
            .any(|argument| !self.inline_safe_argument_expression(argument))
        {
            return Ok(false);
        }
        let result_local = self.allocate_temp_local();
        self.emit_inline_user_function_summary_with_explicit_call_frame(
            &user_function,
            &expanded_arguments,
            &Expression::Undefined,
            result_local,
        )
    }

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
            Statement::Assign { name, value }
            | Statement::Var { name, value }
            | Statement::Let { name, value, .. }
                if Self::is_direct_arguments_identifier(value) =>
            {
                if !targets.contains(name) {
                    targets.push(name.clone());
                }
            }
            Statement::Expression(Expression::Assign { name, value })
                if Self::is_direct_arguments_identifier(value) =>
            {
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
        let mut active_functions = HashSet::new();
        self.sync_direct_arguments_assignments_from_static_user_call_inner(
            user_function,
            arguments,
            &mut active_functions,
        );
    }

    fn sync_direct_arguments_assignments_from_static_user_call_inner(
        &mut self,
        user_function: &UserFunction,
        arguments: &[Expression],
        active_functions: &mut HashSet<String>,
    ) {
        if !active_functions.insert(user_function.name.clone()) {
            return;
        }
        let Some(declaration) = self.resolve_registered_function_declaration(&user_function.name)
        else {
            active_functions.remove(&user_function.name);
            return;
        };
        let body = declaration.body.clone();
        let parameter_defaults = user_function.parameter_defaults.clone();
        if !user_function.lexical_this
            && !user_function.params.iter().any(|param| {
                param == "arguments"
                    || scoped_binding_source_name(param)
                        .is_some_and(|source_name| source_name == "arguments")
            })
        {
            let mut targets = Vec::new();
            for default in parameter_defaults.iter().flatten() {
                Self::collect_direct_arguments_assignment_targets_from_expression(
                    default,
                    &mut targets,
                );
            }
            for statement in &body {
                Self::collect_direct_arguments_assignment_targets_from_statement(
                    statement,
                    &mut targets,
                );
            }
            if !targets.is_empty() {
                let arguments_binding =
                    ArgumentsValueBinding::for_user_function(user_function, arguments.to_vec());
                for target in targets {
                    if user_function.scope_bindings.contains(&target) {
                        continue;
                    }
                    self.backend
                        .sync_global_arguments_binding(&target, Some(arguments_binding.clone()));
                    self.backend
                        .shared_global_semantics
                        .values
                        .sync_arguments_binding(&target, Some(arguments_binding.clone()));
                }
            }
        }

        for default in parameter_defaults.iter().flatten() {
            self.sync_nested_direct_arguments_assignments_from_static_expression(
                Some(&user_function.name),
                default,
                active_functions,
            );
        }
        for statement in &body {
            self.sync_nested_direct_arguments_assignments_from_static_statement(
                Some(&user_function.name),
                statement,
                active_functions,
            );
        }
        active_functions.remove(&user_function.name);
    }

    fn sync_nested_direct_arguments_assignments_from_static_call(
        &mut self,
        current_function_name: Option<&str>,
        callee: &Expression,
        arguments: &[CallArgument],
        active_functions: &mut HashSet<String>,
    ) {
        self.sync_nested_direct_arguments_assignments_from_static_expression(
            current_function_name,
            callee,
            active_functions,
        );
        for argument in arguments {
            self.sync_nested_direct_arguments_assignments_from_static_expression(
                current_function_name,
                argument.expression(),
                active_functions,
            );
        }
        let Some(LocalFunctionBinding::User(function_name)) = self
            .resolve_function_binding_from_expression_with_context(callee, current_function_name)
        else {
            return;
        };
        let Some(user_function) = self.user_function(&function_name).cloned() else {
            return;
        };
        let expanded_arguments = self.expand_call_arguments(arguments);
        self.sync_direct_arguments_assignments_from_static_user_call_inner(
            &user_function,
            &expanded_arguments,
            active_functions,
        );
    }

    fn sync_nested_direct_arguments_assignments_from_static_expression(
        &mut self,
        current_function_name: Option<&str>,
        expression: &Expression,
        active_functions: &mut HashSet<String>,
    ) {
        if Self::call_is_promise_like_chain(expression) {
            return;
        }
        match expression {
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => self.sync_nested_direct_arguments_assignments_from_static_expression(
                current_function_name,
                value,
                active_functions,
            ),
            Expression::Member { object, property }
            | Expression::AssignMember {
                object,
                property,
                value: _,
            } => {
                self.sync_nested_direct_arguments_assignments_from_static_expression(
                    current_function_name,
                    object,
                    active_functions,
                );
                self.sync_nested_direct_arguments_assignments_from_static_expression(
                    current_function_name,
                    property,
                    active_functions,
                );
                if let Expression::AssignMember { value, .. } = expression {
                    self.sync_nested_direct_arguments_assignments_from_static_expression(
                        current_function_name,
                        value,
                        active_functions,
                    );
                }
            }
            Expression::SuperMember { property } => {
                self.sync_nested_direct_arguments_assignments_from_static_expression(
                    current_function_name,
                    property,
                    active_functions,
                );
            }
            Expression::AssignSuperMember { property, value } => {
                self.sync_nested_direct_arguments_assignments_from_static_expression(
                    current_function_name,
                    property,
                    active_functions,
                );
                self.sync_nested_direct_arguments_assignments_from_static_expression(
                    current_function_name,
                    value,
                    active_functions,
                );
            }
            Expression::Binary { left, right, .. } => {
                self.sync_nested_direct_arguments_assignments_from_static_expression(
                    current_function_name,
                    left,
                    active_functions,
                );
                self.sync_nested_direct_arguments_assignments_from_static_expression(
                    current_function_name,
                    right,
                    active_functions,
                );
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.sync_nested_direct_arguments_assignments_from_static_expression(
                    current_function_name,
                    condition,
                    active_functions,
                );
                self.sync_nested_direct_arguments_assignments_from_static_expression(
                    current_function_name,
                    then_expression,
                    active_functions,
                );
                self.sync_nested_direct_arguments_assignments_from_static_expression(
                    current_function_name,
                    else_expression,
                    active_functions,
                );
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    self.sync_nested_direct_arguments_assignments_from_static_expression(
                        current_function_name,
                        expression,
                        active_functions,
                    );
                }
            }
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                self.sync_nested_direct_arguments_assignments_from_static_call(
                    current_function_name,
                    callee,
                    arguments,
                    active_functions,
                );
            }
            Expression::Array(elements) => {
                for element in elements {
                    match element {
                        ArrayElement::Expression(value) | ArrayElement::Spread(value) => {
                            self.sync_nested_direct_arguments_assignments_from_static_expression(
                                current_function_name,
                                value,
                                active_functions,
                            );
                        }
                    }
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            self.sync_nested_direct_arguments_assignments_from_static_expression(
                                current_function_name,
                                key,
                                active_functions,
                            );
                            self.sync_nested_direct_arguments_assignments_from_static_expression(
                                current_function_name,
                                value,
                                active_functions,
                            );
                        }
                        ObjectEntry::Getter { key, getter } => {
                            self.sync_nested_direct_arguments_assignments_from_static_expression(
                                current_function_name,
                                key,
                                active_functions,
                            );
                            self.sync_nested_direct_arguments_assignments_from_static_expression(
                                current_function_name,
                                getter,
                                active_functions,
                            );
                        }
                        ObjectEntry::Setter { key, setter } => {
                            self.sync_nested_direct_arguments_assignments_from_static_expression(
                                current_function_name,
                                key,
                                active_functions,
                            );
                            self.sync_nested_direct_arguments_assignments_from_static_expression(
                                current_function_name,
                                setter,
                                active_functions,
                            );
                        }
                        ObjectEntry::Spread(value) => {
                            self.sync_nested_direct_arguments_assignments_from_static_expression(
                                current_function_name,
                                value,
                                active_functions,
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

    fn sync_nested_direct_arguments_assignments_from_static_statement(
        &mut self,
        current_function_name: Option<&str>,
        statement: &Statement,
        active_functions: &mut HashSet<String>,
    ) {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => {
                for statement in body {
                    self.sync_nested_direct_arguments_assignments_from_static_statement(
                        current_function_name,
                        statement,
                        active_functions,
                    );
                }
            }
            Statement::Var { value, .. }
            | Statement::Let { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Throw(value)
            | Statement::Return(value)
            | Statement::Yield { value }
            | Statement::YieldDelegate { value }
            | Statement::Expression(value) => {
                self.sync_nested_direct_arguments_assignments_from_static_expression(
                    current_function_name,
                    value,
                    active_functions,
                );
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                self.sync_nested_direct_arguments_assignments_from_static_expression(
                    current_function_name,
                    object,
                    active_functions,
                );
                self.sync_nested_direct_arguments_assignments_from_static_expression(
                    current_function_name,
                    property,
                    active_functions,
                );
                self.sync_nested_direct_arguments_assignments_from_static_expression(
                    current_function_name,
                    value,
                    active_functions,
                );
            }
            Statement::Print { values } => {
                for value in values {
                    self.sync_nested_direct_arguments_assignments_from_static_expression(
                        current_function_name,
                        value,
                        active_functions,
                    );
                }
            }
            Statement::With { object, body } => {
                self.sync_nested_direct_arguments_assignments_from_static_expression(
                    current_function_name,
                    object,
                    active_functions,
                );
                for statement in body {
                    self.sync_nested_direct_arguments_assignments_from_static_statement(
                        current_function_name,
                        statement,
                        active_functions,
                    );
                }
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.sync_nested_direct_arguments_assignments_from_static_expression(
                    current_function_name,
                    condition,
                    active_functions,
                );
                for statement in then_branch.iter().chain(else_branch) {
                    self.sync_nested_direct_arguments_assignments_from_static_statement(
                        current_function_name,
                        statement,
                        active_functions,
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
                    self.sync_nested_direct_arguments_assignments_from_static_statement(
                        current_function_name,
                        statement,
                        active_functions,
                    );
                }
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                self.sync_nested_direct_arguments_assignments_from_static_expression(
                    current_function_name,
                    discriminant,
                    active_functions,
                );
                for case in cases {
                    for statement in &case.body {
                        self.sync_nested_direct_arguments_assignments_from_static_statement(
                            current_function_name,
                            statement,
                            active_functions,
                        );
                    }
                }
            }
            Statement::For {
                init,
                condition,
                update,
                break_hook,
                body,
                ..
            } => {
                for statement in init.iter().chain(body) {
                    self.sync_nested_direct_arguments_assignments_from_static_statement(
                        current_function_name,
                        statement,
                        active_functions,
                    );
                }
                for expression in condition.iter().chain(update).chain(break_hook) {
                    self.sync_nested_direct_arguments_assignments_from_static_expression(
                        current_function_name,
                        expression,
                        active_functions,
                    );
                }
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
                self.sync_nested_direct_arguments_assignments_from_static_expression(
                    current_function_name,
                    condition,
                    active_functions,
                );
                if let Some(break_hook) = break_hook {
                    self.sync_nested_direct_arguments_assignments_from_static_expression(
                        current_function_name,
                        break_hook,
                        active_functions,
                    );
                }
                for statement in body {
                    self.sync_nested_direct_arguments_assignments_from_static_statement(
                        current_function_name,
                        statement,
                        active_functions,
                    );
                }
            }
            Statement::Break { .. } | Statement::Continue { .. } => {}
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

        if name == "Date" {
            for argument in arguments {
                match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        self.emit_numeric_expression(expression)?;
                        self.state.emission.output.instructions.push(0x1a);
                    }
                }
            }
            self.push_i32_const(JS_TYPEOF_STRING_TAG);
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

        if matches!(name, "Math.ceil" | "Math.floor") {
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
                | "Math.ceil"
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
            "Reflect.apply" => {
                return self.emit_reflect_apply_call(arguments);
            }
            "Reflect.construct" => {
                return self.emit_reflect_construct_call(arguments);
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
            "Object.defineProperties" => {
                return self.emit_object_define_properties_call(
                    &object_identifier,
                    &Expression::String("defineProperties".to_string()),
                    arguments,
                );
            }
            "Object.freeze" => {
                return self.emit_object_freeze_call(
                    &object_identifier,
                    &Expression::String("freeze".to_string()),
                    arguments,
                );
            }
            "Object.isFrozen" => {
                return self.emit_object_is_frozen_call(
                    &object_identifier,
                    &Expression::String("isFrozen".to_string()),
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
            "Object.isSealed" => {
                return self.emit_object_is_sealed_call(
                    &object_identifier,
                    &Expression::String("isSealed".to_string()),
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
            "Object.seal" => {
                return self.emit_object_seal_call(
                    &object_identifier,
                    &Expression::String("seal".to_string()),
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
            "Proxy.revocable" => {
                self.discard_call_arguments(arguments)?;
                self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
                return Ok(true);
            }
            "Reflect.deleteProperty" => {
                return self.emit_reflect_delete_property_call(
                    &reflect_identifier,
                    &Expression::String("deleteProperty".to_string()),
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
            "Reflect.get" => {
                return self.emit_reflect_get_call(
                    &reflect_identifier,
                    &Expression::String("get".to_string()),
                    arguments,
                );
            }
            "Reflect.getOwnPropertyDescriptor" => {
                return self.emit_object_get_own_property_descriptor_call(
                    &reflect_identifier,
                    &Expression::String("getOwnPropertyDescriptor".to_string()),
                    arguments,
                );
            }
            "Reflect.getPrototypeOf" => {
                return self.emit_object_get_prototype_of_call(
                    &reflect_identifier,
                    &Expression::String("getPrototypeOf".to_string()),
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
            "Reflect.isExtensible" => {
                return self.emit_object_is_extensible_call(
                    &reflect_identifier,
                    &Expression::String("isExtensible".to_string()),
                    arguments,
                );
            }
            "Reflect.ownKeys" => {
                return self.emit_object_array_builtin_call(
                    &reflect_identifier,
                    &Expression::String("ownKeys".to_string()),
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
            "Reflect.set" => {
                return self.emit_reflect_set_call(
                    &reflect_identifier,
                    &Expression::String("set".to_string()),
                    arguments,
                );
            }
            "Reflect.setPrototypeOf" => {
                return self.emit_object_set_prototype_of_call(
                    &reflect_identifier,
                    &Expression::String("setPrototypeOf".to_string()),
                    arguments,
                );
            }
            "Date.now" => {
                for argument in arguments {
                    match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.emit_numeric_expression(expression)?;
                            self.state.emission.output.instructions.push(0x1a);
                        }
                    }
                }
                let synthetic_now =
                    (self.state.emission.output.instructions.len() as f64 + 1.0) * 101.0;
                self.emit_numeric_expression(&Expression::Number(synthetic_now))?;
                return Ok(true);
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

        if name == "__ayyDynamicImport" {
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

        if name == "__ayyImportMeta" {
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

        if name == "Promise.resolve"
            && let Some(CallArgument::Expression(expression)) = arguments.first()
            && self.emit_promise_resolve_user_call_argument_side_effects(expression)?
        {
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            return Ok(true);
        }

        let Some(result_tag) = (match name {
            "Promise.resolve" | "Promise.reject" | "Promise.withResolvers" => {
                Some(JS_TYPEOF_OBJECT_TAG)
            }
            "Number" => Some(JS_TYPEOF_NUMBER_TAG),
            "Boolean" => Some(JS_TYPEOF_BOOLEAN_TAG),
            "Object" | "Array" | "ArrayBuffer" | "SharedArrayBuffer" | "DataView" | "RegExp"
            | "Map" | "Set" | "Error" | "EvalError" | "RangeError" | "ReferenceError"
            | "SyntaxError" | "TypeError" | "URIError" | "AggregateError" | "SuppressedError"
            | "Promise" | "WeakMap" | "WeakRef" | "WeakSet" | "Uint8Array" | "Int8Array"
            | "Uint16Array" | "Int16Array" | "Uint32Array" | "Int32Array" | "Float32Array"
            | "Float64Array" | "Uint8ClampedArray" | "BigInt64Array" | "BigUint64Array" => {
                Some(JS_TYPEOF_OBJECT_TAG)
            }
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
