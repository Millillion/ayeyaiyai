use super::*;

thread_local! {
    static STATIC_OBJECT_PROTOTYPE_RESOLUTION_STACK: std::cell::RefCell<Vec<Expression>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

impl<'a> FunctionCompiler<'a> {
    fn simple_generator_prefix_assigned_prototype_expression(
        &self,
        expression: &Expression,
        prototype_owner: &str,
    ) -> Option<Expression> {
        let prefix_effects = self.simple_generator_call_time_prefix_effects(expression)?;
        let mut assigned_prototype = None;
        for effect in prefix_effects {
            let (object, property, value) = match effect {
                Statement::AssignMember {
                    object,
                    property,
                    value,
                } => (object, property, value),
                Statement::Expression(Expression::AssignMember {
                    object,
                    property,
                    value,
                }) => (*object, *property, *value),
                _ => continue,
            };
            if !matches!(property, Expression::String(name) if name == "prototype") {
                continue;
            }
            let targets_owner = matches!(
                &object,
                Expression::Identifier(name) if name == prototype_owner
            ) || self
                .resolve_function_binding_from_expression(&object)
                .and_then(|binding| self.function_prototype_binding_owner_name(&binding))
                .is_some_and(|owner| owner == prototype_owner);
            if targets_owner {
                assigned_prototype = Some(value);
            }
        }
        assigned_prototype
    }

    fn generator_iterator_prototype_after_call_time_prefix(
        &self,
        expression: &Expression,
        user_function: &UserFunction,
        prototype_owner: &str,
    ) -> Option<Expression> {
        if !self.simple_generator_call_time_prefix_may_assign_prototype(user_function) {
            return None;
        }
        let assigned_prototype = self
            .simple_generator_prefix_assigned_prototype_expression(expression, prototype_owner)?;
        let materialized = self.materialize_static_expression(&assigned_prototype);
        if self
            .resolve_static_primitive_expression_with_context(
                &materialized,
                self.current_function_name(),
            )
            .is_some()
        {
            return Self::generator_intrinsic_default_prototype_expression(user_function.kind);
        }
        Some(Self::normalize_static_object_prototype_target_expression(
            &materialized,
        ))
    }

    fn simple_generator_call_time_prefix_may_assign_prototype(
        &self,
        user_function: &UserFunction,
    ) -> bool {
        user_function
            .parameter_defaults
            .iter()
            .flatten()
            .any(Self::expression_may_assign_prototype_member)
            || self
                .resolve_registered_function_declaration(&user_function.name)
                .is_some_and(|function| {
                    function
                        .body
                        .iter()
                        .any(Self::statement_may_assign_prototype_member)
                })
    }

    fn expression_may_assign_prototype_member(expression: &Expression) -> bool {
        match expression {
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                matches!(property.as_ref(), Expression::String(name) if name == "prototype")
                    || Self::expression_may_assign_prototype_member(object)
                    || Self::expression_may_assign_prototype_member(property)
                    || Self::expression_may_assign_prototype_member(value)
            }
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                    Self::expression_may_assign_prototype_member(expression)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    Self::expression_may_assign_prototype_member(key)
                        || Self::expression_may_assign_prototype_member(value)
                }
                ObjectEntry::Getter { key, getter } => {
                    Self::expression_may_assign_prototype_member(key)
                        || Self::expression_may_assign_prototype_member(getter)
                }
                ObjectEntry::Setter { key, setter } => {
                    Self::expression_may_assign_prototype_member(key)
                        || Self::expression_may_assign_prototype_member(setter)
                }
                ObjectEntry::Spread(expression) => {
                    Self::expression_may_assign_prototype_member(expression)
                }
            }),
            Expression::Member { object, property }
            | Expression::Binary {
                left: object,
                right: property,
                ..
            } => {
                Self::expression_may_assign_prototype_member(object)
                    || Self::expression_may_assign_prototype_member(property)
            }
            Expression::Assign { value, .. }
            | Expression::AssignSuperMember { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => Self::expression_may_assign_prototype_member(value),
            Expression::SuperMember { property } => {
                Self::expression_may_assign_prototype_member(property)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::expression_may_assign_prototype_member(condition)
                    || Self::expression_may_assign_prototype_member(then_expression)
                    || Self::expression_may_assign_prototype_member(else_expression)
            }
            Expression::Sequence(expressions) => expressions
                .iter()
                .any(Self::expression_may_assign_prototype_member),
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                Self::expression_may_assign_prototype_member(callee)
                    || arguments.iter().any(|argument| {
                        Self::expression_may_assign_prototype_member(argument.expression())
                    })
            }
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::NewTarget
            | Expression::Identifier(_)
            | Expression::This
            | Expression::Sent
            | Expression::Update { .. } => false,
        }
    }

    fn statement_may_assign_prototype_member(statement: &Statement) -> bool {
        match statement {
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                matches!(property, Expression::String(name) if name == "prototype")
                    || Self::expression_may_assign_prototype_member(object)
                    || Self::expression_may_assign_prototype_member(property)
                    || Self::expression_may_assign_prototype_member(value)
            }
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. }
            | Statement::While { body, .. }
            | Statement::DoWhile { body, .. } => {
                body.iter().any(Self::statement_may_assign_prototype_member)
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                Self::expression_may_assign_prototype_member(condition)
                    || then_branch
                        .iter()
                        .any(Self::statement_may_assign_prototype_member)
                    || else_branch
                        .iter()
                        .any(Self::statement_may_assign_prototype_member)
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => body
                .iter()
                .chain(catch_setup.iter())
                .chain(catch_body.iter())
                .any(Self::statement_may_assign_prototype_member),
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                Self::expression_may_assign_prototype_member(discriminant)
                    || cases.iter().any(|case| {
                        case.test
                            .as_ref()
                            .is_some_and(Self::expression_may_assign_prototype_member)
                            || case
                                .body
                                .iter()
                                .any(Self::statement_may_assign_prototype_member)
                    })
            }
            Statement::For {
                init,
                condition,
                update,
                break_hook,
                body,
                ..
            } => {
                init.iter().any(Self::statement_may_assign_prototype_member)
                    || condition
                        .as_ref()
                        .is_some_and(Self::expression_may_assign_prototype_member)
                    || update
                        .as_ref()
                        .is_some_and(Self::expression_may_assign_prototype_member)
                    || break_hook
                        .as_ref()
                        .is_some_and(Self::expression_may_assign_prototype_member)
                    || body.iter().any(Self::statement_may_assign_prototype_member)
            }
            Statement::Var { value, .. }
            | Statement::Let { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Expression(value)
            | Statement::Throw(value)
            | Statement::Return(value)
            | Statement::Yield { value }
            | Statement::YieldDelegate { value } => {
                Self::expression_may_assign_prototype_member(value)
            }
            Statement::Print { values } => values
                .iter()
                .any(Self::expression_may_assign_prototype_member),
            Statement::Break { .. } | Statement::Continue { .. } => false,
        }
    }

    fn with_static_object_prototype_resolution_guard<T>(
        &self,
        expression: &Expression,
        f: impl FnOnce(&Self) -> Option<T>,
    ) -> Option<T> {
        if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
            let depth = STATIC_OBJECT_PROTOTYPE_RESOLUTION_STACK.with(|stack| stack.borrow().len());
            if depth < 32 {
                eprintln!(
                    "runtime_shadow_prototype_resolve depth={depth} expression={expression:?}"
                );
            }
        }
        let reentered = STATIC_OBJECT_PROTOTYPE_RESOLUTION_STACK.with(|stack| {
            stack
                .borrow()
                .iter()
                .any(|visited| static_expression_matches(visited, expression))
        });
        if reentered {
            if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                eprintln!("runtime_shadow_prototype_resolve_reentered expression={expression:?}");
            }
            return None;
        }

        STATIC_OBJECT_PROTOTYPE_RESOLUTION_STACK.with(|stack| {
            stack.borrow_mut().push(expression.clone());
        });
        let result = f(self);
        STATIC_OBJECT_PROTOTYPE_RESOLUTION_STACK.with(|stack| {
            stack.borrow_mut().pop();
        });
        result
    }

    fn resolve_static_user_function_call_return_expression(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Option<Expression> {
        let LocalFunctionBinding::User(function_name) =
            self.resolve_function_binding_from_expression(callee)?
        else {
            return None;
        };
        let capture_source_bindings =
            self.resolve_function_expression_capture_slots(callee)
                .map(|capture_slots| {
                    capture_slots
                        .into_iter()
                        .map(|(capture_name, slot_name)| {
                            (
                                capture_name,
                                self.snapshot_bound_capture_slot_expression(&slot_name),
                            )
                        })
                        .collect::<HashMap<_, _>>()
                });
        self.resolve_static_return_expression_from_user_function_call(
            &function_name,
            arguments,
            capture_source_bindings.as_ref(),
        )
    }

    fn resolve_static_call_callee_user_function(
        &self,
        callee: &Expression,
    ) -> Option<UserFunction> {
        if let Some(user_function) = self.resolve_user_function_from_expression(callee) {
            return Some(user_function.clone());
        }
        if let Expression::Identifier(name) = callee
            && let Some(value) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
                .or_else(|| self.global_value_binding(name))
                .filter(|value| !static_expression_matches(value, callee))
            && let Some(user_function) = self.resolve_static_call_callee_user_function(value)
        {
            return Some(user_function);
        }
        let materialized = self.materialize_static_expression(callee);
        if !static_expression_matches(&materialized, callee) {
            return self.resolve_static_call_callee_user_function(&materialized);
        }
        None
    }

    fn static_object_binding_property_value(
        &self,
        name: &str,
        property: &Expression,
    ) -> Option<&Expression> {
        self.state
            .speculation
            .static_semantics
            .local_object_binding(name)
            .and_then(|object_binding| object_binding_lookup_value(object_binding, property))
            .or_else(|| {
                self.global_object_binding(name).and_then(|object_binding| {
                    object_binding_lookup_value(object_binding, property)
                })
            })
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_object_prototype_expression(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        self.with_static_object_prototype_resolution_guard(expression, |this| {
            if let Expression::New { callee, .. } = expression {
                if let Some(binding) = this.resolve_function_binding_from_expression(callee) {
                    let prototype_owner = this.function_prototype_binding_owner_name(&binding)?;
                    return Some(Self::prototype_member_expression(&prototype_owner));
                }
                if let Expression::Identifier(name) = callee.as_ref() {
                    return Some(Self::prototype_member_expression(name));
                }
                return None;
            }
            if let Expression::Call { callee, .. } = expression
                && this
                    .resolve_static_call_callee_user_function(callee.as_ref())
                    .is_some_and(|user_function| {
                        user_function.is_async() && !user_function.is_generator()
                    })
            {
                return Some(Self::prototype_member_expression("Promise"));
            }
            if let Expression::Call { callee, .. } = expression
                && let Some(user_function) =
                    this.resolve_static_call_callee_user_function(callee.as_ref())
                && user_function.is_generator()
            {
                if let Expression::Identifier(callee_name) = callee.as_ref()
                    && let Some(assigned_prototype) = this
                        .static_object_binding_property_value(
                            callee_name,
                            &Expression::String("prototype".to_string()),
                        )
                        .cloned()
                {
                    if std::env::var_os("AYY_TRACE_OBJECT_PROTOTYPES").is_some() {
                        eprintln!(
                            "object_prototype_generator_call callee={callee_name} assigned_prototype={assigned_prototype:?}"
                        );
                    }
                    if this
                        .resolve_static_primitive_expression_with_context(
                            &assigned_prototype,
                            this.current_function_name(),
                        )
                        .is_some()
                    {
                        return Self::generator_intrinsic_default_prototype_expression(
                            user_function.kind,
                        );
                    }
                    if this
                        .resolve_static_object_identity_expression(&assigned_prototype)
                        .is_some()
                    {
                        return Some(Self::normalize_static_object_prototype_target_expression(
                            &assigned_prototype,
                        ));
                    }
                }
                let function_binding = this.resolve_function_binding_from_expression(callee);
                let prototype_owner = function_binding
                    .as_ref()
                    .and_then(|binding| this.function_prototype_binding_owner_name(binding))
                    .unwrap_or_else(|| user_function.name.clone());
                if let Some(prototype) = this.generator_iterator_prototype_after_call_time_prefix(
                    expression,
                    &user_function,
                    &prototype_owner,
                ) {
                    return Some(prototype);
                }
                let prototype = Self::prototype_member_expression(&prototype_owner);
                if this
                    .resolve_static_primitive_expression_with_context(
                        &prototype,
                        this.current_function_name(),
                    )
                    .is_some()
                {
                    return Self::generator_intrinsic_default_prototype_expression(
                        user_function.kind,
                    );
                }
                return Some(prototype);
            }
            if let Some(snapshot_result) = this
                .resolve_call_snapshot_result_expression(expression)
                .filter(|resolved| !static_expression_matches(resolved, expression))
            {
                return this.resolve_static_object_prototype_expression(&snapshot_result);
            }
            if matches!(expression, Expression::This)
                && let Some(current_function_name) = this.current_function_name()
                && current_function_name.starts_with("__ayy_class_ctor_")
                && let Some(function) = this.current_user_function_declaration()
                && let Some(self_binding) = function.self_binding.as_deref()
            {
                return Some(Self::prototype_member_expression(self_binding));
            }
            if let Expression::Identifier(name) = expression
                && let Some(prototype) = this.global_object_prototype_expression(name)
            {
                return Some(prototype.clone());
            }
            if let Expression::Identifier(name) = expression
                && let Some(value) = this
                    .state
                    .speculation
                    .static_semantics
                    .local_value_binding(name)
                    .or_else(|| this.global_value_binding(name))
                    .filter(|value| {
                        !matches!(value, Expression::Identifier(alias) if alias == name)
                    })
                && let Some(prototype) = this.resolve_static_object_prototype_expression(value)
            {
                return Some(prototype);
            }
            if this.expression_is_known_array_value(expression) {
                return Some(Self::prototype_member_expression("Array"));
            }
            if this.expression_is_known_promise_instance_for_instanceof(expression) {
                return Some(Self::prototype_member_expression("Promise"));
            }
            let preserve_tracked_expression = match expression {
                Expression::Identifier(name) => {
                    this.backend.global_has_prototype_object_binding(name)
                        || this.global_object_prototype_expression(name).is_some()
                }
                Expression::Member { object, property } if matches!(property.as_ref(), Expression::String(name) if name == "prototype") => {
                    match object.as_ref() {
                        Expression::Identifier(name) => this
                            .static_object_binding_property_value(
                                name,
                                &Expression::String("prototype".to_string()),
                            )
                            .is_some(),
                        _ => false,
                    }
                }
                _ => false,
            };
            if !preserve_tracked_expression
                && let Some(resolved) = this
                    .resolve_bound_alias_expression(expression)
                    .filter(|resolved| !static_expression_matches(resolved, expression))
            {
                return this.resolve_static_object_prototype_expression(&resolved);
            }

            match expression {
                Expression::Sequence(expressions) => {
                    let last = expressions.last()?;
                    return this.resolve_static_object_prototype_expression(last);
                }
                Expression::Identifier(name) => {
                    if let Some(resolved) = this
                        .resolve_static_class_init_local_alias_expression(name)
                        .filter(|resolved| !static_expression_matches(
                            resolved,
                            expression,
                        ))
                    {
                        return this.resolve_static_object_prototype_expression(&resolved);
                    }
                    if let Some(prototype) = this.global_object_prototype_expression(name) {
                        return Some(prototype.clone());
                    }
                    if let Some(value) = this
                        .state
                        .speculation
                        .static_semantics
                        .local_value_binding(name)
                        .or_else(|| this.global_value_binding(name))
                        .filter(
                            |value| !matches!(value, Expression::Identifier(alias) if alias == name),
                        )
                        && let Some(prototype) =
                            this.resolve_static_object_prototype_expression(value)
                    {
                        return Some(prototype);
                    }
                    if let Some(prototype) =
                        Self::builtin_constructor_object_prototype_expression(name)
                    {
                        return Some(prototype);
                    }
                    if let Some(binding) = this.resolve_function_binding_from_expression(expression)
                    {
                        let prototype_owner = match &binding {
                            LocalFunctionBinding::User(function_name) => this
                                .user_function(function_name)
                                .map(|user_function| match user_function.kind {
                                    FunctionKind::Generator => "GeneratorFunction",
                                    FunctionKind::Async => "AsyncFunction",
                                    FunctionKind::AsyncGenerator => "AsyncGeneratorFunction",
                                    FunctionKind::Ordinary => "Function",
                                })
                                .unwrap_or("Function"),
                            LocalFunctionBinding::Builtin(_) => "Function",
                        };
                        return Some(Self::prototype_member_expression(prototype_owner));
                    }
                }
                Expression::Object(_) => {
                    return Some(
                        object_literal_prototype_expression(expression)
                            .unwrap_or_else(|| Self::prototype_member_expression("Object")),
                    );
                }
                Expression::Member { object, property } if matches!(property.as_ref(), Expression::String(name) if name == "prototype") =>
                {
                    let Expression::Identifier(name) = object.as_ref() else {
                        return None;
                    };
                    let mut prototype_owner_names = vec![name.clone()];
                    if let Some(Expression::Identifier(resolved_name)) = this
                        .resolve_bound_alias_expression(object)
                        .filter(|resolved| !static_expression_matches(resolved, object))
                    {
                        prototype_owner_names.push(resolved_name);
                    }
                    if let Some(Expression::Identifier(resolved_name)) = this
                        .state
                        .speculation
                        .static_semantics
                        .local_value_binding(name)
                        .or_else(|| this.global_value_binding(name))
                        .map(|value| {
                            this.resolve_bound_alias_expression(value)
                                .filter(|resolved| !static_expression_matches(resolved, value))
                                .unwrap_or_else(|| this.materialize_static_expression(value))
                        })
                    {
                        prototype_owner_names.push(resolved_name);
                    }
                    if let Some(resolved_name) = this.resolve_static_class_init_constructor_alias(name)
                    {
                        prototype_owner_names.push(resolved_name);
                    }
                    if let Some(Expression::Identifier(resolved_name)) =
                        this.resolve_static_class_init_local_alias_expression(name)
                    {
                        prototype_owner_names.push(resolved_name);
                    }
                    prototype_owner_names.sort();
                    prototype_owner_names.dedup();
                    for prototype_owner_name in &prototype_owner_names {
                        let prototype_key = format!("{prototype_owner_name}.prototype");
                        if let Some(prototype) =
                            this.global_object_prototype_expression(&prototype_key)
                        {
                            return Some(prototype.clone());
                        }
                    }
                    if let Some(Expression::Identifier(resolved_name)) =
                        this.resolve_static_class_init_local_alias_expression(name)
                    {
                        return Some(Self::prototype_member_expression(&resolved_name));
                    }
                    if let Some(value) = this
                        .static_object_binding_property_value(
                            name,
                            &Expression::String("prototype".to_string()),
                        )
                        .cloned()
                    {
                        if let Some(prototype) =
                            this.resolve_static_object_prototype_expression(&value)
                        {
                            return Some(prototype);
                        }
                        if this
                            .resolve_static_primitive_expression_with_context(
                                &value,
                                this.current_function_name(),
                            )
                            .is_some()
                        {
                            return None;
                        }
                    }
                    if let Some(prototype) =
                        Self::builtin_prototype_object_prototype_expression(name)
                    {
                        return Some(prototype);
                    }
                    if let Some(user_function) = this.resolve_user_function_from_expression(object)
                        && user_function.is_generator()
                    {
                        return Self::generator_intrinsic_default_prototype_expression(
                            user_function.kind,
                        );
                    }
                    if this
                        .resolve_function_binding_from_expression(object)
                        .is_some()
                        || matches!(infer_call_result_kind(name), Some(_))
                    {
                        return Some(Self::prototype_member_expression("Object"));
                    }
                }
                Expression::Call { callee, .. } => {
                    let Expression::Call { arguments, .. } = expression else {
                        unreachable!("matched call expression above");
                    };
                    if let Expression::Member { object, property } = callee.as_ref()
                        && matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
                        && matches!(property.as_ref(), Expression::String(name) if name == "getPrototypeOf")
                        && let Some(
                            CallArgument::Expression(target) | CallArgument::Spread(target),
                        ) = arguments.first()
                        && let Some(prototype) =
                            this.resolve_static_object_prototype_expression(target)
                    {
                        return this.resolve_static_object_prototype_expression(&prototype);
                    }
                    if let Expression::Member { object, property } = callee.as_ref()
                        && matches!(property.as_ref(), Expression::String(name) if name == "slice")
                        && let Some(prototype) =
                            this.resolve_static_object_prototype_expression(object)
                    {
                        return Some(prototype);
                    }
                    if let Expression::Member { object, property } = callee.as_ref()
                        && matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
                        && matches!(property.as_ref(), Expression::String(name) if name == "create")
                    {
                        if let Some(
                            CallArgument::Expression(prototype) | CallArgument::Spread(prototype),
                        ) = arguments.first()
                        {
                            let prototype = this
                                .resolve_bound_alias_expression(prototype)
                                .filter(|resolved| !static_expression_matches(resolved, prototype))
                                .unwrap_or_else(|| this.materialize_static_expression(prototype));
                            return Some(Self::normalize_static_object_prototype_target_expression(
                                &prototype,
                            ));
                        }
                    }
                    if matches!(
                        callee.as_ref(),
                        Expression::Member { object, property }
                            if matches!(object.as_ref(), Expression::Identifier(name) if name == "Promise")
                                && matches!(property.as_ref(), Expression::String(name) if name == "resolve")
                    ) {
                        return Some(Self::prototype_member_expression("Promise"));
                    }
                    if this
                        .resolve_static_call_callee_user_function(callee.as_ref())
                        .is_some_and(|user_function| {
                            user_function.is_async() && !user_function.is_generator()
                        })
                    {
                        return Some(Self::prototype_member_expression("Promise"));
                    }
                    if let Some(user_function) =
                        this.resolve_static_call_callee_user_function(callee.as_ref())
                        && user_function.is_generator()
                    {
                        if let Expression::Identifier(callee_name) = callee.as_ref()
                            && let Some(assigned_prototype) = this
                                .static_object_binding_property_value(
                                    callee_name,
                                    &Expression::String("prototype".to_string()),
                                )
                                .cloned()
                        {
                            if std::env::var_os("AYY_TRACE_OBJECT_PROTOTYPES").is_some() {
                                eprintln!(
                                    "object_prototype_generator_call callee={callee_name} assigned_prototype={assigned_prototype:?}"
                                );
                            }
                            if this
                                .resolve_static_primitive_expression_with_context(
                                    &assigned_prototype,
                                    this.current_function_name(),
                                )
                                .is_some()
                            {
                                return Self::generator_intrinsic_default_prototype_expression(
                                    user_function.kind,
                                );
                            }
                            if this
                                .resolve_static_object_identity_expression(&assigned_prototype)
                                .is_some()
                            {
                                return Some(Self::normalize_static_object_prototype_target_expression(
                                    &assigned_prototype,
                                ));
                            }
                        }
                        let direct_prototype_expression = Expression::Member {
                            object: Box::new(callee.as_ref().clone()),
                            property: Box::new(Expression::String("prototype".to_string())),
                        };
                        let materialized_direct_prototype =
                            this.materialize_static_expression(&direct_prototype_expression);
                        if std::env::var_os("AYY_TRACE_OBJECT_PROTOTYPES").is_some() {
                            eprintln!(
                                "object_prototype_generator_call direct={direct_prototype_expression:?} materialized={materialized_direct_prototype:?}"
                            );
                        }
                        if !static_expression_matches(
                            &materialized_direct_prototype,
                            &direct_prototype_expression,
                        ) {
                            if this
                                .resolve_static_primitive_expression_with_context(
                                    &materialized_direct_prototype,
                                    this.current_function_name(),
                                )
                                .is_some()
                            {
                                return Self::generator_intrinsic_default_prototype_expression(
                                    user_function.kind,
                                );
                            }
                            if this
                                .resolve_static_object_identity_expression(
                                    &materialized_direct_prototype,
                                )
                                .is_some()
                            {
                                return Some(Self::normalize_static_object_prototype_target_expression(
                                    &materialized_direct_prototype,
                                ));
                            }
                        }
                        let function_binding =
                            this.resolve_function_binding_from_expression(callee.as_ref());
                        let prototype_owner = function_binding
                            .as_ref()
                            .and_then(|binding| this.function_prototype_binding_owner_name(binding))
                            .unwrap_or_else(|| user_function.name.clone());
                        if let Some(prototype) = this
                            .generator_iterator_prototype_after_call_time_prefix(
                                expression,
                                &user_function,
                                &prototype_owner,
                            )
                        {
                            return Some(prototype);
                        }
                        let prototype = Self::prototype_member_expression(&prototype_owner);
                        if this
                            .resolve_static_primitive_expression_with_context(
                                &prototype,
                                this.current_function_name(),
                            )
                            .is_some()
                        {
                            return Self::generator_intrinsic_default_prototype_expression(
                                user_function.kind,
                            );
                        }
                        return Some(prototype);
                    }
                    if let Some(returned_expression) = this
                        .resolve_static_user_function_call_return_expression(callee, arguments)
                        .filter(|returned| !static_expression_matches(returned, expression))
                        && let Some(prototype) =
                            this.resolve_static_object_prototype_expression(&returned_expression)
                    {
                        return Some(prototype);
                    }
                    let Expression::Identifier(name) = callee.as_ref() else {
                        return None;
                    };
                    if native_error_runtime_value(name).is_some() {
                        return Some(Self::prototype_member_expression(name));
                    }
                }
                _ => {}
            }

            let materialized = this.materialize_static_expression(expression);
            if !static_expression_matches(&materialized, expression) {
                return this.resolve_static_object_prototype_expression(&materialized);
            }
            None
        })
    }
}
