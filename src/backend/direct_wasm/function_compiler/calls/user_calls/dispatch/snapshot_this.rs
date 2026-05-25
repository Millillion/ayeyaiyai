use super::*;

thread_local! {
    static STATIC_THIS_MEMBER_WRITE_EXPRESSION_DEPTH: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
}

struct StaticThisMemberWriteExpressionGuard;
struct StaticThisMemberWriteExpressionRootGuard {
    previous_depth: usize,
}

impl StaticThisMemberWriteExpressionGuard {
    fn enter() -> (Self, bool) {
        let should_substitute = STATIC_THIS_MEMBER_WRITE_EXPRESSION_DEPTH.with(|depth| {
            let current = depth.get();
            depth.set(current.saturating_add(1));
            current == 0
        });
        (Self, should_substitute)
    }
}

impl Drop for StaticThisMemberWriteExpressionGuard {
    fn drop(&mut self) {
        STATIC_THIS_MEMBER_WRITE_EXPRESSION_DEPTH.with(|depth| {
            depth.set(depth.get().saturating_sub(1));
        });
    }
}

impl StaticThisMemberWriteExpressionRootGuard {
    fn enter() -> Self {
        let previous_depth = STATIC_THIS_MEMBER_WRITE_EXPRESSION_DEPTH.with(|depth| {
            let previous_depth = depth.get();
            depth.set(0);
            previous_depth
        });
        Self { previous_depth }
    }
}

impl Drop for StaticThisMemberWriteExpressionRootGuard {
    fn drop(&mut self) {
        STATIC_THIS_MEMBER_WRITE_EXPRESSION_DEPTH.with(|depth| {
            depth.set(self.previous_depth);
        });
    }
}

impl<'a> FunctionCompiler<'a> {
    fn receiver_value_is_class_object(value: &Expression) -> bool {
        match value {
            Expression::Identifier(name) => {
                name.starts_with("__ayy_class_ctor_") || name.starts_with("__ayy_class_expr_")
            }
            Expression::Call { callee, .. } => {
                matches!(callee.as_ref(), Expression::Identifier(name) if name.starts_with("__ayy_class_init_"))
            }
            _ => false,
        }
    }

    fn class_receiver_shadow_owner_fallback(&self, name: &str) -> Option<String> {
        self.state
            .speculation
            .static_semantics
            .local_value_binding(name)
            .or_else(|| self.global_value_binding(name))
            .is_some_and(Self::receiver_value_is_class_object)
            .then(|| name.to_string())
    }

    pub(in crate::backend::direct_wasm) fn receiver_shadow_updated_via_parameter_writebacks(
        &self,
        this_expression: &Expression,
        writebacks: &[(String, String, Option<ObjectValueBinding>)],
    ) -> bool {
        self.resolve_user_function_call_receiver_shadow_owner(this_expression)
            .as_deref()
            .is_some_and(|target_owner| {
                writebacks
                    .iter()
                    .any(|(_, source_owner, _)| source_owner == target_owner)
            })
    }

    fn sync_receiver_metadata_from_runtime_shadow(
        &mut self,
        this_expression: &Expression,
        target_owner: &str,
        updated_receiver_binding: &ObjectValueBinding,
    ) {
        self.sync_runtime_object_property_shadow_static_metadata_from_binding(
            target_owner,
            updated_receiver_binding,
        );
        let updated_receiver_expression = object_binding_to_expression(updated_receiver_binding);
        let resolved_identifier_name = match this_expression {
            Expression::Identifier(name) => self
                .resolve_current_local_binding(name)
                .map(|(resolved_name, _)| resolved_name)
                .filter(|resolved_name| resolved_name != name),
            _ => None,
        };
        match this_expression {
            Expression::Identifier(name) => {
                if let Some(resolved_name) = resolved_identifier_name.as_deref() {
                    self.update_local_value_binding(resolved_name, &updated_receiver_expression);
                    self.update_local_object_binding(resolved_name, &updated_receiver_expression);
                }
                self.update_local_value_binding(name, &updated_receiver_expression);
                self.update_local_object_binding(name, &updated_receiver_expression);
                if self.binding_name_is_global(name)
                    || self.global_has_binding(name)
                    || self.global_has_implicit_binding(name)
                {
                    self.update_static_global_assignment_metadata(
                        name,
                        &updated_receiver_expression,
                    );
                }
            }
            Expression::This => {
                self.update_local_value_binding("this", &updated_receiver_expression);
                self.update_local_object_binding("this", &updated_receiver_expression);
            }
            _ => {}
        }
    }

    fn object_binding_contains_private_brand_marker(
        &self,
        object_binding: &ObjectValueBinding,
        private_brand_binding: &str,
    ) -> bool {
        let expected_value = self.materialize_static_expression(&Expression::Identifier(
            private_brand_binding.to_string(),
        ));
        ordered_object_property_names(object_binding)
            .into_iter()
            .any(|property_name| {
                self.resolve_object_binding_property_value(
                    object_binding,
                    &Expression::String(property_name),
                )
                .is_some_and(|value| {
                    let materialized_value = self.materialize_static_expression(&value);
                    static_expression_matches(&materialized_value, &expected_value)
                        || static_expression_matches(&expected_value, &materialized_value)
                })
            })
    }

    pub(in crate::backend::direct_wasm) fn user_function_call_allows_static_this_shadow_commit(
        &self,
        user_function: &UserFunction,
        this_expression: &Expression,
    ) -> bool {
        if user_function.lexical_this
            || !self.user_function_mentions_private_member_access(user_function)
        {
            return true;
        }
        let Some(private_brand_binding) = user_function.private_brand_binding.as_deref() else {
            return false;
        };
        self.resolve_user_function_call_receiver_shadow_owner(this_expression)
            .and_then(|owner| self.resolve_runtime_shadow_object_binding(&owner))
            .is_some_and(|object_binding| {
                self.object_binding_contains_private_brand_marker(
                    &object_binding,
                    private_brand_binding,
                )
            })
            || self
                .resolve_object_binding_from_expression(this_expression)
                .is_some_and(|object_binding| {
                    self.object_binding_contains_private_brand_marker(
                        &object_binding,
                        private_brand_binding,
                    )
                })
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_snapshot_this_expression(
        &self,
        this_expression: &Expression,
    ) -> Expression {
        if matches!(this_expression, Expression::Identifier(name) if name == "globalThis" && self.is_unshadowed_builtin_identifier(name))
        {
            return Expression::This;
        }

        if !matches!(this_expression, Expression::This)
            && self
                .resolve_static_reference_identity_key(this_expression)
                .is_some()
            && self
                .resolve_object_binding_from_expression(this_expression)
                .is_some()
        {
            return this_expression.clone();
        }

        let resolved_this = self
            .resolve_bound_alias_expression(this_expression)
            .filter(|resolved| !static_expression_matches(resolved, this_expression))
            .unwrap_or_else(|| this_expression.clone());

        if !matches!(resolved_this, Expression::This) {
            if self
                .resolve_static_reference_identity_key(&resolved_this)
                .is_some()
                && self
                    .resolve_object_binding_from_expression(&resolved_this)
                    .is_some()
            {
                return resolved_this;
            }
            return match &resolved_this {
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
                    })
                    .unwrap_or(resolved_this),
                _ => self.materialize_static_expression(&resolved_this),
            };
        }

        self.resolve_object_binding_from_expression(&Expression::This)
            .map(|binding| object_binding_to_expression(&binding))
            .or_else(|| {
                self.state
                    .speculation
                    .static_semantics
                    .local_value_binding("this")
                    .cloned()
            })
            .or_else(|| {
                self.backend
                    .global_semantics
                    .values
                    .value_bindings
                    .get("this")
                    .cloned()
            })
            .unwrap_or(resolved_this)
    }

    pub(in crate::backend::direct_wasm) fn resolve_user_function_call_receiver_shadow_owner(
        &self,
        this_expression: &Expression,
    ) -> Option<String> {
        match this_expression {
            Expression::Identifier(name) => {
                return self
                    .runtime_object_property_shadow_owner_name_for_identifier(name)
                    .or_else(|| self.class_receiver_shadow_owner_fallback(name));
            }
            Expression::This => return Some("this".to_string()),
            _ => {}
        }
        let resolved_this = self
            .resolve_bound_alias_expression(this_expression)
            .filter(|resolved| !static_expression_matches(resolved, this_expression))
            .unwrap_or_else(|| this_expression.clone());
        match resolved_this {
            Expression::Identifier(name) => self
                .runtime_object_property_shadow_owner_name_for_identifier(&name)
                .or_else(|| self.class_receiver_shadow_owner_fallback(&name)),
            Expression::This => Some("this".to_string()),
            _ => None,
        }
    }

    fn collect_static_this_member_write_property_names_from_expression(
        &self,
        expression: &Expression,
        property_names: &mut BTreeSet<String>,
    ) {
        match expression {
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                if matches!(object.as_ref(), Expression::This) {
                    let property = self.canonical_object_property_expression(property);
                    if let Some(property_name) = static_property_name_from_expression(&property) {
                        property_names.insert(property_name);
                    }
                }
                self.collect_static_this_member_write_property_names_from_expression(
                    object,
                    property_names,
                );
                self.collect_static_this_member_write_property_names_from_expression(
                    property,
                    property_names,
                );
                self.collect_static_this_member_write_property_names_from_expression(
                    value,
                    property_names,
                );
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => {
                self.collect_static_this_member_write_property_names_from_expression(
                    value,
                    property_names,
                );
            }
            Expression::AssignSuperMember { property, value } => {
                let materialized_property = self.canonical_object_property_expression(property);
                if let Some(property_name) =
                    static_property_name_from_expression(&materialized_property)
                {
                    property_names.insert(property_name);
                }
                self.collect_static_this_member_write_property_names_from_expression(
                    property,
                    property_names,
                );
                self.collect_static_this_member_write_property_names_from_expression(
                    value,
                    property_names,
                );
            }
            Expression::Member { object, property } => {
                self.collect_static_this_member_write_property_names_from_expression(
                    object,
                    property_names,
                );
                self.collect_static_this_member_write_property_names_from_expression(
                    property,
                    property_names,
                );
            }
            Expression::SuperMember { property } => {
                self.collect_static_this_member_write_property_names_from_expression(
                    property,
                    property_names,
                );
            }
            Expression::Binary { left, right, .. } => {
                self.collect_static_this_member_write_property_names_from_expression(
                    left,
                    property_names,
                );
                self.collect_static_this_member_write_property_names_from_expression(
                    right,
                    property_names,
                );
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.collect_static_this_member_write_property_names_from_expression(
                    condition,
                    property_names,
                );
                self.collect_static_this_member_write_property_names_from_expression(
                    then_expression,
                    property_names,
                );
                self.collect_static_this_member_write_property_names_from_expression(
                    else_expression,
                    property_names,
                );
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    self.collect_static_this_member_write_property_names_from_expression(
                        expression,
                        property_names,
                    );
                }
            }
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                self.collect_static_this_member_write_property_names_from_expression(
                    callee,
                    property_names,
                );
                for argument in arguments {
                    match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.collect_static_this_member_write_property_names_from_expression(
                                expression,
                                property_names,
                            );
                        }
                    }
                }
            }
            Expression::Array(elements) => {
                for element in elements {
                    match element {
                        ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                            self.collect_static_this_member_write_property_names_from_expression(
                                expression,
                                property_names,
                            );
                        }
                    }
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            self.collect_static_this_member_write_property_names_from_expression(
                                key,
                                property_names,
                            );
                            self.collect_static_this_member_write_property_names_from_expression(
                                value,
                                property_names,
                            );
                        }
                        ObjectEntry::Getter { key, getter } => {
                            self.collect_static_this_member_write_property_names_from_expression(
                                key,
                                property_names,
                            );
                            self.collect_static_this_member_write_property_names_from_expression(
                                getter,
                                property_names,
                            );
                        }
                        ObjectEntry::Setter { key, setter } => {
                            self.collect_static_this_member_write_property_names_from_expression(
                                key,
                                property_names,
                            );
                            self.collect_static_this_member_write_property_names_from_expression(
                                setter,
                                property_names,
                            );
                        }
                        ObjectEntry::Spread(expression) => {
                            self.collect_static_this_member_write_property_names_from_expression(
                                expression,
                                property_names,
                            );
                        }
                    }
                }
            }
            Expression::Update { .. }
            | Expression::Identifier(_)
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::This
            | Expression::NewTarget
            | Expression::Sent => {}
        }
    }

    fn collect_static_this_member_write_property_names_from_statement(
        &self,
        statement: &Statement,
        property_names: &mut BTreeSet<String>,
    ) {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. } => {
                for statement in body {
                    self.collect_static_this_member_write_property_names_from_statement(
                        statement,
                        property_names,
                    );
                }
            }
            Statement::Expression(expression)
            | Statement::Return(expression)
            | Statement::Throw(expression)
            | Statement::Yield { value: expression }
            | Statement::YieldDelegate { value: expression }
            | Statement::Var {
                value: expression, ..
            }
            | Statement::Let {
                value: expression, ..
            } => {
                self.collect_static_this_member_write_property_names_from_expression(
                    expression,
                    property_names,
                );
            }
            Statement::Print { values } => {
                for value in values {
                    self.collect_static_this_member_write_property_names_from_expression(
                        value,
                        property_names,
                    );
                }
            }
            Statement::Assign { value, .. } => {
                self.collect_static_this_member_write_property_names_from_expression(
                    value,
                    property_names,
                );
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                if matches!(object, Expression::This) {
                    let property = self.canonical_object_property_expression(property);
                    if let Some(property_name) = static_property_name_from_expression(&property) {
                        property_names.insert(property_name);
                    }
                }
                self.collect_static_this_member_write_property_names_from_expression(
                    object,
                    property_names,
                );
                self.collect_static_this_member_write_property_names_from_expression(
                    property,
                    property_names,
                );
                self.collect_static_this_member_write_property_names_from_expression(
                    value,
                    property_names,
                );
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.collect_static_this_member_write_property_names_from_expression(
                    condition,
                    property_names,
                );
                for statement in then_branch {
                    self.collect_static_this_member_write_property_names_from_statement(
                        statement,
                        property_names,
                    );
                }
                for statement in else_branch {
                    self.collect_static_this_member_write_property_names_from_statement(
                        statement,
                        property_names,
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
                self.collect_static_this_member_write_property_names_from_expression(
                    condition,
                    property_names,
                );
                if let Some(break_hook) = break_hook {
                    self.collect_static_this_member_write_property_names_from_expression(
                        break_hook,
                        property_names,
                    );
                }
                for statement in body {
                    self.collect_static_this_member_write_property_names_from_statement(
                        statement,
                        property_names,
                    );
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
                for statement in init {
                    self.collect_static_this_member_write_property_names_from_statement(
                        statement,
                        property_names,
                    );
                }
                if let Some(condition) = condition {
                    self.collect_static_this_member_write_property_names_from_expression(
                        condition,
                        property_names,
                    );
                }
                if let Some(update) = update {
                    self.collect_static_this_member_write_property_names_from_expression(
                        update,
                        property_names,
                    );
                }
                if let Some(break_hook) = break_hook {
                    self.collect_static_this_member_write_property_names_from_expression(
                        break_hook,
                        property_names,
                    );
                }
                for statement in body {
                    self.collect_static_this_member_write_property_names_from_statement(
                        statement,
                        property_names,
                    );
                }
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                for statement in body {
                    self.collect_static_this_member_write_property_names_from_statement(
                        statement,
                        property_names,
                    );
                }
                for statement in catch_setup {
                    self.collect_static_this_member_write_property_names_from_statement(
                        statement,
                        property_names,
                    );
                }
                for statement in catch_body {
                    self.collect_static_this_member_write_property_names_from_statement(
                        statement,
                        property_names,
                    );
                }
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                self.collect_static_this_member_write_property_names_from_expression(
                    discriminant,
                    property_names,
                );
                for case in cases {
                    if let Some(test) = &case.test {
                        self.collect_static_this_member_write_property_names_from_expression(
                            test,
                            property_names,
                        );
                    }
                    for statement in &case.body {
                        self.collect_static_this_member_write_property_names_from_statement(
                            statement,
                            property_names,
                        );
                    }
                }
            }
            Statement::Break { .. } | Statement::Continue { .. } => {}
        }
    }

    fn user_function_static_this_member_write_property_names(
        &self,
        user_function: &UserFunction,
    ) -> BTreeSet<String> {
        let Some(function) = self.resolve_registered_function_declaration(&user_function.name)
        else {
            return BTreeSet::new();
        };
        let mut property_names = BTreeSet::new();
        for statement in &function.body {
            self.collect_static_this_member_write_property_names_from_statement(
                statement,
                &mut property_names,
            );
        }
        property_names
    }

    fn static_function_call_argument_bindings(
        &self,
        function_name: &str,
        arguments: &[CallArgument],
    ) -> HashMap<String, Expression> {
        let expanded_arguments = self.expand_call_arguments(arguments);
        let mut bindings = HashMap::new();
        if let Some(declaration) = self.resolve_registered_function_declaration(function_name) {
            for (index, parameter) in declaration.params.iter().enumerate() {
                let value = if parameter.rest {
                    Expression::Array(
                        expanded_arguments
                            .iter()
                            .skip(index)
                            .cloned()
                            .map(ArrayElement::Expression)
                            .collect(),
                    )
                } else {
                    expanded_arguments
                        .get(index)
                        .cloned()
                        .unwrap_or(Expression::Undefined)
                };
                bindings.insert(parameter.name.clone(), value);
            }
        } else if let Some(user_function) = self.user_function(function_name) {
            for (index, param_name) in user_function.params.iter().enumerate() {
                bindings.insert(
                    param_name.clone(),
                    expanded_arguments
                        .get(index)
                        .cloned()
                        .unwrap_or(Expression::Undefined),
                );
            }
        }
        bindings
    }

    fn resolve_static_user_function_name_for_expression(
        &self,
        expression: &Expression,
        current_function_name: Option<&str>,
        function_aliases: &HashMap<String, String>,
    ) -> Option<String> {
        if let Expression::Identifier(name) = expression
            && let Some(function_name) = function_aliases.get(name)
        {
            return Some(function_name.clone());
        }
        match self.resolve_function_binding_from_expression_with_context(
            expression,
            current_function_name,
        ) {
            Some(LocalFunctionBinding::User(function_name)) => Some(function_name),
            _ => None,
        }
    }

    fn update_static_function_alias(
        &self,
        name: &str,
        value: &Expression,
        current_function_name: Option<&str>,
        function_aliases: &mut HashMap<String, String>,
    ) {
        if let Some(function_name) = self.resolve_static_user_function_name_for_expression(
            value,
            current_function_name,
            function_aliases,
        ) {
            function_aliases.insert(name.to_string(), function_name);
        } else {
            function_aliases.remove(name);
        }
    }

    fn update_static_value_alias(
        &self,
        name: &str,
        value: &Expression,
        static_bindings: &mut HashMap<String, Expression>,
    ) {
        match value {
            Expression::This => {
                static_bindings.insert(name.to_string(), Expression::This);
            }
            _ => {
                static_bindings.remove(name);
            }
        }
    }

    fn collect_static_this_member_write_property_values_from_user_function_call(
        &self,
        function_name: &str,
        arguments: &[CallArgument],
        property_values: &mut BTreeMap<String, Expression>,
        visited_functions: &mut HashSet<String>,
        inherited_static_bindings: &HashMap<String, Expression>,
    ) {
        let _root_guard = StaticThisMemberWriteExpressionRootGuard::enter();
        if !visited_functions.insert(function_name.to_string()) {
            return;
        }
        let Some(function) = self.resolve_registered_function_declaration(function_name) else {
            visited_functions.remove(function_name);
            return;
        };
        let body = function.body.clone();
        let argument_bindings =
            self.static_function_call_argument_bindings(function_name, arguments);
        let mut static_bindings = inherited_static_bindings.clone();
        static_bindings.extend(argument_bindings);
        let mut function_aliases = HashMap::new();
        for statement in &body {
            self.collect_static_this_member_write_property_values_from_statement_with_context(
                statement,
                Some(function_name),
                property_values,
                &mut function_aliases,
                &mut static_bindings,
                visited_functions,
            );
            if Self::statement_unconditionally_transfers_control(statement) {
                break;
            }
        }
        visited_functions.remove(function_name);
    }

    fn expression_makes_static_super_receiver_nonextensible(
        &self,
        expression: &Expression,
        current_function_name: Option<&str>,
    ) -> bool {
        let Expression::Call { callee, arguments } = expression else {
            return false;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return false;
        };
        let is_nonextensible_builtin = matches!(
            (object.as_ref(), property.as_ref()),
            (Expression::Identifier(object_name), Expression::String(property_name))
                if (object_name == "Object"
                    && matches!(
                        property_name.as_str(),
                        "freeze" | "seal" | "preventExtensions"
                    ))
                    || (object_name == "Reflect" && property_name == "preventExtensions")
        );
        if !is_nonextensible_builtin {
            return false;
        }
        let Some(CallArgument::Expression(target) | CallArgument::Spread(target)) =
            arguments.first()
        else {
            return false;
        };
        if matches!(target, Expression::This) {
            return true;
        }
        let Some(current_function_name) = current_function_name else {
            return false;
        };
        let Some(home_object_name) =
            self.resolve_home_object_name_for_function(current_function_name)
        else {
            return false;
        };
        matches!(target, Expression::Identifier(target_name) if target_name == &home_object_name)
    }

    fn collect_static_direct_super_member_write_property_values_from_expression(
        &self,
        expression: &Expression,
        current_function_name: Option<&str>,
        receiver_extensible: &mut bool,
        property_values: &mut BTreeMap<String, Expression>,
        static_bindings: &mut HashMap<String, Expression>,
        visited_functions: &mut HashSet<String>,
    ) {
        match expression {
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    self.collect_static_direct_super_member_write_property_values_from_expression(
                        expression,
                        current_function_name,
                        receiver_extensible,
                        property_values,
                        static_bindings,
                        visited_functions,
                    );
                }
            }
            Expression::AssignSuperMember { property, value } => {
                let property = self.substitute_expression_bindings(property, static_bindings);
                let value = self.substitute_expression_bindings(value, static_bindings);
                self.collect_static_direct_super_member_write_property_values_from_expression(
                    &property,
                    current_function_name,
                    receiver_extensible,
                    property_values,
                    static_bindings,
                    visited_functions,
                );
                self.collect_static_direct_super_member_write_property_values_from_expression(
                    &value,
                    current_function_name,
                    receiver_extensible,
                    property_values,
                    static_bindings,
                    visited_functions,
                );
                if self.collect_static_super_member_write_property_values_from_setter(
                    &property,
                    &value,
                    current_function_name,
                    property_values,
                    visited_functions,
                ) {
                    return;
                }
                let property = self.canonical_object_property_expression(&property);
                if let Some(property_name) = static_property_name_from_expression(&property)
                    && (*receiver_extensible || property_values.contains_key(&property_name))
                {
                    property_values.insert(
                        property_name,
                        self.reference_preserving_static_value_expression(&value),
                    );
                }
            }
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                for expression in [object.as_ref(), property.as_ref(), value.as_ref()] {
                    self.collect_static_direct_super_member_write_property_values_from_expression(
                        expression,
                        current_function_name,
                        receiver_extensible,
                        property_values,
                        static_bindings,
                        visited_functions,
                    );
                }
            }
            Expression::Assign { name, value } => {
                self.collect_static_direct_super_member_write_property_values_from_expression(
                    value,
                    current_function_name,
                    receiver_extensible,
                    property_values,
                    static_bindings,
                    visited_functions,
                );
                self.update_static_value_alias(name, value, static_bindings);
            }
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                self.collect_static_direct_super_member_write_property_values_from_expression(
                    callee,
                    current_function_name,
                    receiver_extensible,
                    property_values,
                    static_bindings,
                    visited_functions,
                );
                for argument in arguments {
                    match argument {
                        CallArgument::Expression(argument) | CallArgument::Spread(argument) => {
                            self.collect_static_direct_super_member_write_property_values_from_expression(
                                argument,
                                current_function_name,
                                receiver_extensible,
                                property_values,
                                static_bindings,
                                visited_functions,
                            );
                        }
                    }
                }
                if self.expression_makes_static_super_receiver_nonextensible(
                    expression,
                    current_function_name,
                ) {
                    *receiver_extensible = false;
                }
            }
            Expression::Array(elements) => {
                for element in elements {
                    let expression = match element {
                        ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                            expression
                        }
                    };
                    self.collect_static_direct_super_member_write_property_values_from_expression(
                        expression,
                        current_function_name,
                        receiver_extensible,
                        property_values,
                        static_bindings,
                        visited_functions,
                    );
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            for expression in [key, value] {
                                self.collect_static_direct_super_member_write_property_values_from_expression(
                                    expression,
                                    current_function_name,
                                    receiver_extensible,
                                    property_values,
                                    static_bindings,
                                    visited_functions,
                                );
                            }
                        }
                        ObjectEntry::Getter { key, getter } => {
                            for expression in [key, getter] {
                                self.collect_static_direct_super_member_write_property_values_from_expression(
                                    expression,
                                    current_function_name,
                                    receiver_extensible,
                                    property_values,
                                    static_bindings,
                                    visited_functions,
                                );
                            }
                        }
                        ObjectEntry::Setter { key, setter } => {
                            for expression in [key, setter] {
                                self.collect_static_direct_super_member_write_property_values_from_expression(
                                    expression,
                                    current_function_name,
                                    receiver_extensible,
                                    property_values,
                                    static_bindings,
                                    visited_functions,
                                );
                            }
                        }
                        ObjectEntry::Spread(expression) => {
                            self.collect_static_direct_super_member_write_property_values_from_expression(
                                expression,
                                current_function_name,
                                receiver_extensible,
                                property_values,
                                static_bindings,
                                visited_functions,
                            );
                        }
                    }
                }
            }
            Expression::Member { object, property }
            | Expression::Binary {
                left: object,
                right: property,
                ..
            } => {
                for expression in [object.as_ref(), property.as_ref()] {
                    self.collect_static_direct_super_member_write_property_values_from_expression(
                        expression,
                        current_function_name,
                        receiver_extensible,
                        property_values,
                        static_bindings,
                        visited_functions,
                    );
                }
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                for expression in [
                    condition.as_ref(),
                    then_expression.as_ref(),
                    else_expression.as_ref(),
                ] {
                    self.collect_static_direct_super_member_write_property_values_from_expression(
                        expression,
                        current_function_name,
                        receiver_extensible,
                        property_values,
                        static_bindings,
                        visited_functions,
                    );
                }
            }
            Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::SuperMember { property: value }
            | Expression::Unary {
                expression: value, ..
            } => self.collect_static_direct_super_member_write_property_values_from_expression(
                value,
                current_function_name,
                receiver_extensible,
                property_values,
                static_bindings,
                visited_functions,
            ),
            _ => {}
        }
    }

    fn collect_static_direct_super_member_write_property_values_from_statement(
        &self,
        statement: &Statement,
        current_function_name: Option<&str>,
        receiver_extensible: &mut bool,
        property_values: &mut BTreeMap<String, Expression>,
        function_aliases: &mut HashMap<String, String>,
        static_bindings: &mut HashMap<String, Expression>,
        visited_functions: &mut HashSet<String>,
    ) {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. } => {
                for statement in body {
                    self.collect_static_direct_super_member_write_property_values_from_statement(
                        statement,
                        current_function_name,
                        receiver_extensible,
                        property_values,
                        function_aliases,
                        static_bindings,
                        visited_functions,
                    );
                    if Self::statement_unconditionally_transfers_control(statement) {
                        break;
                    }
                }
            }
            Statement::Expression(expression)
            | Statement::Return(expression)
            | Statement::Throw(expression)
            | Statement::Yield { value: expression }
            | Statement::YieldDelegate { value: expression } => {
                self.collect_static_direct_super_member_write_property_values_from_expression(
                    expression,
                    current_function_name,
                    receiver_extensible,
                    property_values,
                    static_bindings,
                    visited_functions,
                );
            }
            Statement::Var { name, value } | Statement::Let { name, value, .. } => {
                let value = self.substitute_expression_bindings(value, static_bindings);
                self.collect_static_direct_super_member_write_property_values_from_expression(
                    &value,
                    current_function_name,
                    receiver_extensible,
                    property_values,
                    static_bindings,
                    visited_functions,
                );
                self.update_static_function_alias(
                    name,
                    &value,
                    current_function_name,
                    function_aliases,
                );
                self.update_static_value_alias(name, &value, static_bindings);
            }
            Statement::Assign { name, value } => {
                let value = self.substitute_expression_bindings(value, static_bindings);
                self.collect_static_direct_super_member_write_property_values_from_expression(
                    &value,
                    current_function_name,
                    receiver_extensible,
                    property_values,
                    static_bindings,
                    visited_functions,
                );
                self.update_static_function_alias(
                    name,
                    &value,
                    current_function_name,
                    function_aliases,
                );
                self.update_static_value_alias(name, &value, static_bindings);
            }
            Statement::Print { values } => {
                for value in values {
                    self.collect_static_direct_super_member_write_property_values_from_expression(
                        value,
                        current_function_name,
                        receiver_extensible,
                        property_values,
                        static_bindings,
                        visited_functions,
                    );
                }
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                for expression in [object, property, value] {
                    self.collect_static_direct_super_member_write_property_values_from_expression(
                        expression,
                        current_function_name,
                        receiver_extensible,
                        property_values,
                        static_bindings,
                        visited_functions,
                    );
                }
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.collect_static_direct_super_member_write_property_values_from_expression(
                    condition,
                    current_function_name,
                    receiver_extensible,
                    property_values,
                    static_bindings,
                    visited_functions,
                );
                for branch in [then_branch, else_branch] {
                    let mut branch_receiver_extensible = *receiver_extensible;
                    for statement in branch {
                        self.collect_static_direct_super_member_write_property_values_from_statement(
                            statement,
                            current_function_name,
                            &mut branch_receiver_extensible,
                            property_values,
                            function_aliases,
                            static_bindings,
                            visited_functions,
                        );
                    }
                    *receiver_extensible &= branch_receiver_extensible;
                }
            }
            Statement::While {
                condition, body, ..
            }
            | Statement::DoWhile {
                condition, body, ..
            } => {
                self.collect_static_direct_super_member_write_property_values_from_expression(
                    condition,
                    current_function_name,
                    receiver_extensible,
                    property_values,
                    static_bindings,
                    visited_functions,
                );
                for statement in body {
                    self.collect_static_direct_super_member_write_property_values_from_statement(
                        statement,
                        current_function_name,
                        receiver_extensible,
                        property_values,
                        function_aliases,
                        static_bindings,
                        visited_functions,
                    );
                }
            }
            Statement::For {
                init,
                condition,
                update,
                body,
                ..
            } => {
                for statement in init {
                    self.collect_static_direct_super_member_write_property_values_from_statement(
                        statement,
                        current_function_name,
                        receiver_extensible,
                        property_values,
                        function_aliases,
                        static_bindings,
                        visited_functions,
                    );
                }
                if let Some(condition) = condition {
                    self.collect_static_direct_super_member_write_property_values_from_expression(
                        condition,
                        current_function_name,
                        receiver_extensible,
                        property_values,
                        static_bindings,
                        visited_functions,
                    );
                }
                if let Some(update) = update {
                    self.collect_static_direct_super_member_write_property_values_from_expression(
                        update,
                        current_function_name,
                        receiver_extensible,
                        property_values,
                        static_bindings,
                        visited_functions,
                    );
                }
                for statement in body {
                    self.collect_static_direct_super_member_write_property_values_from_statement(
                        statement,
                        current_function_name,
                        receiver_extensible,
                        property_values,
                        function_aliases,
                        static_bindings,
                        visited_functions,
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
                    self.collect_static_direct_super_member_write_property_values_from_statement(
                        statement,
                        current_function_name,
                        receiver_extensible,
                        property_values,
                        function_aliases,
                        static_bindings,
                        visited_functions,
                    );
                }
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                self.collect_static_direct_super_member_write_property_values_from_expression(
                    discriminant,
                    current_function_name,
                    receiver_extensible,
                    property_values,
                    static_bindings,
                    visited_functions,
                );
                for case in cases {
                    if let Some(test) = &case.test {
                        self.collect_static_direct_super_member_write_property_values_from_expression(
                            test,
                            current_function_name,
                            receiver_extensible,
                            property_values,
                            static_bindings,
                            visited_functions,
                        );
                    }
                    for statement in &case.body {
                        self.collect_static_direct_super_member_write_property_values_from_statement(
                            statement,
                            current_function_name,
                            receiver_extensible,
                            property_values,
                            function_aliases,
                            static_bindings,
                            visited_functions,
                        );
                    }
                }
            }
            Statement::Break { .. } | Statement::Continue { .. } => {}
        }
    }

    fn collect_static_direct_super_member_write_property_values_from_user_function_call(
        &self,
        function_name: &str,
        arguments: &[CallArgument],
        property_values: &mut BTreeMap<String, Expression>,
        visited_functions: &mut HashSet<String>,
        inherited_static_bindings: &HashMap<String, Expression>,
    ) {
        if !visited_functions.insert(function_name.to_string()) {
            return;
        }
        let Some(function) = self.resolve_registered_function_declaration(function_name) else {
            visited_functions.remove(function_name);
            return;
        };
        let body = function.body.clone();
        let argument_bindings =
            self.static_function_call_argument_bindings(function_name, arguments);
        let mut static_bindings = inherited_static_bindings.clone();
        static_bindings.extend(argument_bindings);
        let mut function_aliases = HashMap::new();
        let mut receiver_extensible = true;
        for statement in &body {
            self.collect_static_direct_super_member_write_property_values_from_statement(
                statement,
                Some(function_name),
                &mut receiver_extensible,
                property_values,
                &mut function_aliases,
                &mut static_bindings,
                visited_functions,
            );
            if Self::statement_unconditionally_transfers_control(statement) {
                break;
            }
        }
        visited_functions.remove(function_name);
    }

    fn collect_static_this_member_write_property_values_from_setter(
        &self,
        property: &Expression,
        value: &Expression,
        current_function_name: Option<&str>,
        property_values: &mut BTreeMap<String, Expression>,
        visited_functions: &mut HashSet<String>,
    ) -> bool {
        let Some(LocalFunctionBinding::User(setter_name)) = self
            .resolve_member_setter_binding_with_context(
                &Expression::This,
                property,
                current_function_name,
            )
        else {
            return false;
        };
        let arguments = [CallArgument::Expression(
            self.reference_preserving_static_value_expression(value),
        )];
        self.collect_static_this_member_write_property_values_from_user_function_call(
            &setter_name,
            &arguments,
            property_values,
            visited_functions,
            &HashMap::new(),
        );
        true
    }

    fn collect_static_super_member_write_property_values_from_setter(
        &self,
        property: &Expression,
        value: &Expression,
        current_function_name: Option<&str>,
        property_values: &mut BTreeMap<String, Expression>,
        visited_functions: &mut HashSet<String>,
    ) -> bool {
        let Some(super_base) =
            self.resolve_super_base_expression_with_context(current_function_name)
        else {
            return false;
        };
        let property = self.canonical_object_property_expression(property);
        let Some(LocalFunctionBinding::User(setter_name)) = self
            .resolve_member_setter_binding_with_context(
                &super_base,
                &property,
                current_function_name,
            )
        else {
            return false;
        };
        let arguments = [CallArgument::Expression(
            self.reference_preserving_static_value_expression(value),
        )];
        self.collect_static_this_member_write_property_values_from_user_function_call(
            &setter_name,
            &arguments,
            property_values,
            visited_functions,
            &HashMap::new(),
        );
        true
    }

    fn collect_static_this_member_assignment_value(
        &self,
        object: &Expression,
        property: &Expression,
        value: &Expression,
        current_function_name: Option<&str>,
        property_values: &mut BTreeMap<String, Expression>,
        visited_functions: &mut HashSet<String>,
    ) {
        if !matches!(object, Expression::This) {
            return;
        }
        let property = self.canonical_object_property_expression(property);
        if self.collect_static_this_member_write_property_values_from_setter(
            &property,
            value,
            current_function_name,
            property_values,
            visited_functions,
        ) {
            return;
        }
        if let Some(property_name) = static_property_name_from_expression(&property) {
            property_values.insert(
                property_name,
                self.reference_preserving_static_value_expression(value),
            );
        }
    }

    fn collect_static_this_member_write_property_values_from_statement_with_context(
        &self,
        statement: &Statement,
        current_function_name: Option<&str>,
        property_values: &mut BTreeMap<String, Expression>,
        function_aliases: &mut HashMap<String, String>,
        static_bindings: &mut HashMap<String, Expression>,
        visited_functions: &mut HashSet<String>,
    ) {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. } => {
                for statement in body {
                    self.collect_static_this_member_write_property_values_from_statement_with_context(
                        statement,
                        current_function_name,
                        property_values,
                        function_aliases,
                        static_bindings,
                        visited_functions,
                    );
                    if Self::statement_unconditionally_transfers_control(statement) {
                        break;
                    }
                }
            }
            Statement::Expression(expression)
            | Statement::Return(expression)
            | Statement::Throw(expression)
            | Statement::Yield { value: expression }
            | Statement::YieldDelegate { value: expression } => {
                self.collect_static_this_member_write_property_values_from_expression_with_context(
                    expression,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
            }
            Statement::Var { name, value } | Statement::Let { name, value, .. } => {
                let value = self.substitute_expression_bindings(value, static_bindings);
                self.update_static_function_alias(
                    name,
                    &value,
                    current_function_name,
                    function_aliases,
                );
                self.update_static_value_alias(name, &value, static_bindings);
                self.collect_static_this_member_write_property_values_from_expression_with_context(
                    &value,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
            }
            Statement::Assign { name, value } => {
                let value = self.substitute_expression_bindings(value, static_bindings);
                self.collect_static_this_member_write_property_values_from_expression_with_context(
                    &value,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
                self.update_static_function_alias(
                    name,
                    &value,
                    current_function_name,
                    function_aliases,
                );
                self.update_static_value_alias(name, &value, static_bindings);
            }
            Statement::Print { values } => {
                for value in values {
                    self.collect_static_this_member_write_property_values_from_expression_with_context(
                        value,
                        current_function_name,
                        property_values,
                        function_aliases,
                        static_bindings,
                        visited_functions,
                    );
                }
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                let object = self.substitute_expression_bindings(object, static_bindings);
                let property = self.substitute_expression_bindings(property, static_bindings);
                let value = self.substitute_expression_bindings(value, static_bindings);
                self.collect_static_this_member_write_property_values_from_expression_with_context(
                    &object,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
                self.collect_static_this_member_write_property_values_from_expression_with_context(
                    &property,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
                self.collect_static_this_member_write_property_values_from_expression_with_context(
                    &value,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
                self.collect_static_this_member_assignment_value(
                    &object,
                    &property,
                    &value,
                    current_function_name,
                    property_values,
                    visited_functions,
                );
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.collect_static_this_member_write_property_values_from_expression_with_context(
                    condition,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
                for statement in then_branch {
                    self.collect_static_this_member_write_property_values_from_statement_with_context(
                        statement,
                        current_function_name,
                        property_values,
                        function_aliases,
                        static_bindings,
                        visited_functions,
                    );
                }
                for statement in else_branch {
                    self.collect_static_this_member_write_property_values_from_statement_with_context(
                        statement,
                        current_function_name,
                        property_values,
                        function_aliases,
                        static_bindings,
                        visited_functions,
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
                self.collect_static_this_member_write_property_values_from_expression_with_context(
                    condition,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
                if let Some(break_hook) = break_hook {
                    self.collect_static_this_member_write_property_values_from_expression_with_context(
                        break_hook,
                        current_function_name,
                        property_values,
                        function_aliases,
                        static_bindings,
                        visited_functions,
                    );
                }
                for statement in body {
                    self.collect_static_this_member_write_property_values_from_statement_with_context(
                        statement,
                        current_function_name,
                        property_values,
                        function_aliases,
                        static_bindings,
                        visited_functions,
                    );
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
                for statement in init {
                    self.collect_static_this_member_write_property_values_from_statement_with_context(
                        statement,
                        current_function_name,
                        property_values,
                        function_aliases,
                        static_bindings,
                        visited_functions,
                    );
                }
                if let Some(condition) = condition {
                    self.collect_static_this_member_write_property_values_from_expression_with_context(
                        condition,
                        current_function_name,
                        property_values,
                        function_aliases,
                        static_bindings,
                        visited_functions,
                    );
                }
                if let Some(update) = update {
                    self.collect_static_this_member_write_property_values_from_expression_with_context(
                        update,
                        current_function_name,
                        property_values,
                        function_aliases,
                        static_bindings,
                        visited_functions,
                    );
                }
                if let Some(break_hook) = break_hook {
                    self.collect_static_this_member_write_property_values_from_expression_with_context(
                        break_hook,
                        current_function_name,
                        property_values,
                        function_aliases,
                        static_bindings,
                        visited_functions,
                    );
                }
                for statement in body {
                    self.collect_static_this_member_write_property_values_from_statement_with_context(
                        statement,
                        current_function_name,
                        property_values,
                        function_aliases,
                        static_bindings,
                        visited_functions,
                    );
                }
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                for statement in body {
                    self.collect_static_this_member_write_property_values_from_statement_with_context(
                        statement,
                        current_function_name,
                        property_values,
                        function_aliases,
                        static_bindings,
                        visited_functions,
                    );
                }
                for statement in catch_setup {
                    self.collect_static_this_member_write_property_values_from_statement_with_context(
                        statement,
                        current_function_name,
                        property_values,
                        function_aliases,
                        static_bindings,
                        visited_functions,
                    );
                }
                for statement in catch_body {
                    self.collect_static_this_member_write_property_values_from_statement_with_context(
                        statement,
                        current_function_name,
                        property_values,
                        function_aliases,
                        static_bindings,
                        visited_functions,
                    );
                }
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                self.collect_static_this_member_write_property_values_from_expression_with_context(
                    discriminant,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
                for case in cases {
                    if let Some(test) = &case.test {
                        self.collect_static_this_member_write_property_values_from_expression_with_context(
                            test,
                            current_function_name,
                            property_values,
                            function_aliases,
                            static_bindings,
                            visited_functions,
                        );
                    }
                    for statement in &case.body {
                        self.collect_static_this_member_write_property_values_from_statement_with_context(
                            statement,
                            current_function_name,
                            property_values,
                            function_aliases,
                            static_bindings,
                            visited_functions,
                        );
                    }
                }
            }
            Statement::Break { .. } | Statement::Continue { .. } => {}
        }
    }

    fn collect_static_this_member_write_property_values_from_expression_with_context(
        &self,
        expression: &Expression,
        current_function_name: Option<&str>,
        property_values: &mut BTreeMap<String, Expression>,
        function_aliases: &mut HashMap<String, String>,
        static_bindings: &mut HashMap<String, Expression>,
        visited_functions: &mut HashSet<String>,
    ) {
        let (_guard, should_substitute) = StaticThisMemberWriteExpressionGuard::enter();
        let substituted_expression;
        let expression = if should_substitute {
            substituted_expression =
                self.substitute_expression_bindings(expression, static_bindings);
            &substituted_expression
        } else {
            expression
        };
        match &expression {
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.collect_static_this_member_write_property_values_from_expression_with_context(
                    object,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
                self.collect_static_this_member_write_property_values_from_expression_with_context(
                    property,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
                self.collect_static_this_member_write_property_values_from_expression_with_context(
                    value,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
                self.collect_static_this_member_assignment_value(
                    object,
                    property,
                    value,
                    current_function_name,
                    property_values,
                    visited_functions,
                );
            }
            Expression::AssignSuperMember { property, value } => {
                self.collect_static_this_member_write_property_values_from_expression_with_context(
                    property,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
                self.collect_static_this_member_write_property_values_from_expression_with_context(
                    value,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
                self.collect_static_super_member_write_property_values_from_setter(
                    property,
                    value,
                    current_function_name,
                    property_values,
                    visited_functions,
                );
            }
            Expression::Assign { name, value } => {
                self.collect_static_this_member_write_property_values_from_expression_with_context(
                    value,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
                self.update_static_function_alias(
                    name,
                    value,
                    current_function_name,
                    function_aliases,
                );
                self.update_static_value_alias(name, value, static_bindings);
            }
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                self.collect_static_this_member_write_property_values_from_expression_with_context(
                    callee,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
                for argument in arguments {
                    match argument {
                        CallArgument::Expression(argument) | CallArgument::Spread(argument) => {
                            self.collect_static_this_member_write_property_values_from_expression_with_context(
                                argument,
                                current_function_name,
                                property_values,
                                function_aliases,
                                static_bindings,
                                visited_functions,
                            );
                        }
                    }
                }
                if let Some(function_name) = self.resolve_static_user_function_name_for_expression(
                    callee,
                    current_function_name,
                    function_aliases,
                ) {
                    self.collect_static_this_member_write_property_values_from_user_function_call(
                        &function_name,
                        arguments,
                        property_values,
                        visited_functions,
                        static_bindings,
                    );
                }
            }
            Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => {
                self.collect_static_this_member_write_property_values_from_expression_with_context(
                    value,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
            }
            Expression::Member { object, property } => {
                self.collect_static_this_member_write_property_values_from_expression_with_context(
                    object,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
                self.collect_static_this_member_write_property_values_from_expression_with_context(
                    property,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
            }
            Expression::SuperMember { property } => {
                self.collect_static_this_member_write_property_values_from_expression_with_context(
                    property,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
            }
            Expression::Binary { left, right, .. } => {
                self.collect_static_this_member_write_property_values_from_expression_with_context(
                    left,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
                self.collect_static_this_member_write_property_values_from_expression_with_context(
                    right,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.collect_static_this_member_write_property_values_from_expression_with_context(
                    condition,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
                self.collect_static_this_member_write_property_values_from_expression_with_context(
                    then_expression,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
                self.collect_static_this_member_write_property_values_from_expression_with_context(
                    else_expression,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    self.collect_static_this_member_write_property_values_from_expression_with_context(
                        expression,
                        current_function_name,
                        property_values,
                        function_aliases,
                        static_bindings,
                        visited_functions,
                    );
                }
            }
            Expression::Array(elements) => {
                for element in elements {
                    match element {
                        ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                            self.collect_static_this_member_write_property_values_from_expression_with_context(
                                expression,
                                current_function_name,
                                property_values,
                                function_aliases,
                                static_bindings,
                                visited_functions,
                            );
                        }
                    }
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            self.collect_static_this_member_write_property_values_from_expression_with_context(
                                key,
                                current_function_name,
                                property_values,
                                function_aliases,
                                static_bindings,
                                visited_functions,
                            );
                            self.collect_static_this_member_write_property_values_from_expression_with_context(
                                value,
                                current_function_name,
                                property_values,
                                function_aliases,
                                static_bindings,
                                visited_functions,
                            );
                        }
                        ObjectEntry::Getter { key, getter } => {
                            self.collect_static_this_member_write_property_values_from_expression_with_context(
                                key,
                                current_function_name,
                                property_values,
                                function_aliases,
                                static_bindings,
                                visited_functions,
                            );
                            self.collect_static_this_member_write_property_values_from_expression_with_context(
                                getter,
                                current_function_name,
                                property_values,
                                function_aliases,
                                static_bindings,
                                visited_functions,
                            );
                        }
                        ObjectEntry::Setter { key, setter } => {
                            self.collect_static_this_member_write_property_values_from_expression_with_context(
                                key,
                                current_function_name,
                                property_values,
                                function_aliases,
                                static_bindings,
                                visited_functions,
                            );
                            self.collect_static_this_member_write_property_values_from_expression_with_context(
                                setter,
                                current_function_name,
                                property_values,
                                function_aliases,
                                static_bindings,
                                visited_functions,
                            );
                        }
                        ObjectEntry::Spread(expression) => {
                            self.collect_static_this_member_write_property_values_from_expression_with_context(
                                expression,
                                current_function_name,
                                property_values,
                                function_aliases,
                                static_bindings,
                                visited_functions,
                            );
                        }
                    }
                }
            }
            Expression::Update { .. }
            | Expression::Identifier(_)
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::This
            | Expression::NewTarget
            | Expression::Sent => {}
        }
    }

    fn sync_runtime_this_shadow_static_property_values(
        &mut self,
        property_values: &BTreeMap<String, Expression>,
    ) {
        if property_values.is_empty() {
            return;
        }
        let mut object_binding = self
            .resolve_runtime_shadow_object_binding("this")
            .unwrap_or_else(empty_object_value_binding);
        for (property_name, value) in property_values {
            let property = Expression::String(property_name.clone());
            let value = self.reference_preserving_static_value_expression(value);
            object_binding_set_property(&mut object_binding, property.clone(), value.clone());
            for (descriptor_property, descriptor) in &mut object_binding.property_descriptors {
                if *descriptor_property == property
                    && !descriptor.has_get
                    && !descriptor.has_set
                    && descriptor.getter.is_none()
                    && descriptor.setter.is_none()
                {
                    descriptor.value = Some(value.clone());
                }
            }
        }
        self.sync_runtime_object_property_shadow_static_metadata_from_binding(
            "this",
            &object_binding,
        );
        let object_expression = object_binding_to_expression(&object_binding);
        self.update_local_value_binding("this", &object_expression);
        self.update_local_object_binding("this", &object_expression);
    }

    fn user_function_static_this_member_write_property_values_from_arguments(
        &self,
        user_function: &UserFunction,
        argument_expressions: &[Expression],
    ) -> BTreeMap<String, Expression> {
        let mut property_values = BTreeMap::new();
        let mut visited_functions = HashSet::new();
        let arguments = argument_expressions
            .iter()
            .cloned()
            .map(CallArgument::Expression)
            .collect::<Vec<_>>();
        self.collect_static_this_member_write_property_values_from_user_function_call(
            &user_function.name,
            &arguments,
            &mut property_values,
            &mut visited_functions,
            &HashMap::new(),
        );
        self.collect_static_direct_super_member_write_property_values_from_user_function_call(
            &user_function.name,
            &arguments,
            &mut property_values,
            &mut visited_functions,
            &HashMap::new(),
        );
        property_values
    }

    fn static_parameter_writeback_target_name(param_name: &str) -> String {
        format!("__ayy_static_parameter_writeback_target_{param_name}")
    }

    fn expression_is_static_parameter_writeback_target(
        expression: &Expression,
        target_name: &str,
    ) -> bool {
        matches!(expression, Expression::Identifier(name) if name == target_name)
    }

    fn update_static_parameter_writeback_alias(
        name: &str,
        value: &Expression,
        target_name: &str,
        static_bindings: &mut HashMap<String, Expression>,
    ) {
        if Self::expression_is_static_parameter_writeback_target(value, target_name) {
            static_bindings.insert(
                name.to_string(),
                Expression::Identifier(target_name.to_string()),
            );
        } else {
            static_bindings.remove(name);
        }
    }

    fn collect_static_parameter_member_write_property_values_from_setter(
        &self,
        target_expression: &Expression,
        property: &Expression,
        value: &Expression,
        current_function_name: Option<&str>,
        property_values: &mut BTreeMap<String, Expression>,
        visited_functions: &mut HashSet<String>,
    ) -> bool {
        let Some(LocalFunctionBinding::User(setter_name)) = self
            .resolve_member_setter_binding_with_context(
                target_expression,
                property,
                current_function_name,
            )
        else {
            return false;
        };
        let arguments = [CallArgument::Expression(
            self.reference_preserving_static_value_expression(value),
        )];
        self.collect_static_this_member_write_property_values_from_user_function_call(
            &setter_name,
            &arguments,
            property_values,
            visited_functions,
            &HashMap::new(),
        );
        true
    }

    fn collect_static_parameter_member_assignment_value(
        &self,
        object: &Expression,
        property: &Expression,
        value: &Expression,
        actual_target_expression: &Expression,
        target_name: &str,
        current_function_name: Option<&str>,
        property_values: &mut BTreeMap<String, Expression>,
        visited_functions: &mut HashSet<String>,
    ) {
        if !Self::expression_is_static_parameter_writeback_target(object, target_name) {
            return;
        }
        let property = self.canonical_object_property_expression(property);
        if self.collect_static_parameter_member_write_property_values_from_setter(
            actual_target_expression,
            &property,
            value,
            current_function_name,
            property_values,
            visited_functions,
        ) {
            return;
        }
        if let Some(property_name) = static_property_name_from_expression(&property) {
            property_values.insert(
                property_name,
                self.reference_preserving_static_value_expression(value),
            );
        }
    }

    fn collect_static_parameter_member_write_property_values_from_user_function_call(
        &self,
        function_name: &str,
        target_param_name: &str,
        actual_target_expression: &Expression,
        arguments: &[CallArgument],
        property_values: &mut BTreeMap<String, Expression>,
        visited_functions: &mut HashSet<String>,
        inherited_static_bindings: &HashMap<String, Expression>,
    ) {
        let _root_guard = StaticThisMemberWriteExpressionRootGuard::enter();
        let visit_key = format!("{function_name}::{target_param_name}");
        if !visited_functions.insert(visit_key.clone()) {
            return;
        }
        let Some(function) = self.resolve_registered_function_declaration(function_name) else {
            visited_functions.remove(&visit_key);
            return;
        };
        let body = function.body.clone();
        let target_name = Self::static_parameter_writeback_target_name(target_param_name);
        let mut static_bindings = inherited_static_bindings.clone();
        static_bindings
            .extend(self.static_function_call_argument_bindings(function_name, arguments));
        static_bindings.insert(
            target_param_name.to_string(),
            Expression::Identifier(target_name.clone()),
        );
        let mut function_aliases = HashMap::new();
        for statement in &body {
            self.collect_static_parameter_member_write_property_values_from_statement_with_context(
                statement,
                actual_target_expression,
                &target_name,
                Some(function_name),
                property_values,
                &mut function_aliases,
                &mut static_bindings,
                visited_functions,
            );
            if Self::statement_unconditionally_transfers_control(statement) {
                break;
            }
        }
        visited_functions.remove(&visit_key);
    }

    #[allow(clippy::too_many_arguments)]
    fn collect_static_parameter_member_write_property_values_from_statement_with_context(
        &self,
        statement: &Statement,
        actual_target_expression: &Expression,
        target_name: &str,
        current_function_name: Option<&str>,
        property_values: &mut BTreeMap<String, Expression>,
        function_aliases: &mut HashMap<String, String>,
        static_bindings: &mut HashMap<String, Expression>,
        visited_functions: &mut HashSet<String>,
    ) {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. } => {
                for statement in body {
                    self.collect_static_parameter_member_write_property_values_from_statement_with_context(
                        statement,
                        actual_target_expression,
                        target_name,
                        current_function_name,
                        property_values,
                        function_aliases,
                        static_bindings,
                        visited_functions,
                    );
                    if Self::statement_unconditionally_transfers_control(statement) {
                        break;
                    }
                }
            }
            Statement::Expression(expression)
            | Statement::Return(expression)
            | Statement::Throw(expression)
            | Statement::Yield { value: expression }
            | Statement::YieldDelegate { value: expression } => {
                self.collect_static_parameter_member_write_property_values_from_expression_with_context(
                    expression,
                    actual_target_expression,
                    target_name,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
            }
            Statement::Var { name, value } | Statement::Let { name, value, .. } => {
                let value = self.substitute_expression_bindings(value, static_bindings);
                self.update_static_function_alias(
                    name,
                    &value,
                    current_function_name,
                    function_aliases,
                );
                Self::update_static_parameter_writeback_alias(
                    name,
                    &value,
                    target_name,
                    static_bindings,
                );
                self.collect_static_parameter_member_write_property_values_from_expression_with_context(
                    &value,
                    actual_target_expression,
                    target_name,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
            }
            Statement::Assign { name, value } => {
                let value = self.substitute_expression_bindings(value, static_bindings);
                self.collect_static_parameter_member_write_property_values_from_expression_with_context(
                    &value,
                    actual_target_expression,
                    target_name,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
                self.update_static_function_alias(
                    name,
                    &value,
                    current_function_name,
                    function_aliases,
                );
                Self::update_static_parameter_writeback_alias(
                    name,
                    &value,
                    target_name,
                    static_bindings,
                );
            }
            Statement::Print { values } => {
                for value in values {
                    self.collect_static_parameter_member_write_property_values_from_expression_with_context(
                        value,
                        actual_target_expression,
                        target_name,
                        current_function_name,
                        property_values,
                        function_aliases,
                        static_bindings,
                        visited_functions,
                    );
                }
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                let object = self.substitute_expression_bindings(object, static_bindings);
                let property = self.substitute_expression_bindings(property, static_bindings);
                let value = self.substitute_expression_bindings(value, static_bindings);
                self.collect_static_parameter_member_write_property_values_from_expression_with_context(
                    &object,
                    actual_target_expression,
                    target_name,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
                self.collect_static_parameter_member_write_property_values_from_expression_with_context(
                    &property,
                    actual_target_expression,
                    target_name,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
                self.collect_static_parameter_member_write_property_values_from_expression_with_context(
                    &value,
                    actual_target_expression,
                    target_name,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
                self.collect_static_parameter_member_assignment_value(
                    &object,
                    &property,
                    &value,
                    actual_target_expression,
                    target_name,
                    current_function_name,
                    property_values,
                    visited_functions,
                );
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.collect_static_parameter_member_write_property_values_from_expression_with_context(
                    condition,
                    actual_target_expression,
                    target_name,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
                for statement in then_branch.iter().chain(else_branch) {
                    self.collect_static_parameter_member_write_property_values_from_statement_with_context(
                        statement,
                        actual_target_expression,
                        target_name,
                        current_function_name,
                        property_values,
                        function_aliases,
                        static_bindings,
                        visited_functions,
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
                self.collect_static_parameter_member_write_property_values_from_expression_with_context(
                    condition,
                    actual_target_expression,
                    target_name,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
                if let Some(break_hook) = break_hook {
                    self.collect_static_parameter_member_write_property_values_from_expression_with_context(
                        break_hook,
                        actual_target_expression,
                        target_name,
                        current_function_name,
                        property_values,
                        function_aliases,
                        static_bindings,
                        visited_functions,
                    );
                }
                for statement in body {
                    self.collect_static_parameter_member_write_property_values_from_statement_with_context(
                        statement,
                        actual_target_expression,
                        target_name,
                        current_function_name,
                        property_values,
                        function_aliases,
                        static_bindings,
                        visited_functions,
                    );
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
                for statement in init {
                    self.collect_static_parameter_member_write_property_values_from_statement_with_context(
                        statement,
                        actual_target_expression,
                        target_name,
                        current_function_name,
                        property_values,
                        function_aliases,
                        static_bindings,
                        visited_functions,
                    );
                }
                if let Some(condition) = condition {
                    self.collect_static_parameter_member_write_property_values_from_expression_with_context(
                        condition,
                        actual_target_expression,
                        target_name,
                        current_function_name,
                        property_values,
                        function_aliases,
                        static_bindings,
                        visited_functions,
                    );
                }
                if let Some(update) = update {
                    self.collect_static_parameter_member_write_property_values_from_expression_with_context(
                        update,
                        actual_target_expression,
                        target_name,
                        current_function_name,
                        property_values,
                        function_aliases,
                        static_bindings,
                        visited_functions,
                    );
                }
                if let Some(break_hook) = break_hook {
                    self.collect_static_parameter_member_write_property_values_from_expression_with_context(
                        break_hook,
                        actual_target_expression,
                        target_name,
                        current_function_name,
                        property_values,
                        function_aliases,
                        static_bindings,
                        visited_functions,
                    );
                }
                for statement in body {
                    self.collect_static_parameter_member_write_property_values_from_statement_with_context(
                        statement,
                        actual_target_expression,
                        target_name,
                        current_function_name,
                        property_values,
                        function_aliases,
                        static_bindings,
                        visited_functions,
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
                    self.collect_static_parameter_member_write_property_values_from_statement_with_context(
                        statement,
                        actual_target_expression,
                        target_name,
                        current_function_name,
                        property_values,
                        function_aliases,
                        static_bindings,
                        visited_functions,
                    );
                }
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                self.collect_static_parameter_member_write_property_values_from_expression_with_context(
                    discriminant,
                    actual_target_expression,
                    target_name,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
                for case in cases {
                    if let Some(test) = &case.test {
                        self.collect_static_parameter_member_write_property_values_from_expression_with_context(
                            test,
                            actual_target_expression,
                            target_name,
                            current_function_name,
                            property_values,
                            function_aliases,
                            static_bindings,
                            visited_functions,
                        );
                    }
                    for statement in &case.body {
                        self.collect_static_parameter_member_write_property_values_from_statement_with_context(
                            statement,
                            actual_target_expression,
                            target_name,
                            current_function_name,
                            property_values,
                            function_aliases,
                            static_bindings,
                            visited_functions,
                        );
                    }
                }
            }
            Statement::Break { .. } | Statement::Continue { .. } => {}
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn collect_static_parameter_member_write_property_values_from_expression_with_context(
        &self,
        expression: &Expression,
        actual_target_expression: &Expression,
        target_name: &str,
        current_function_name: Option<&str>,
        property_values: &mut BTreeMap<String, Expression>,
        function_aliases: &mut HashMap<String, String>,
        static_bindings: &mut HashMap<String, Expression>,
        visited_functions: &mut HashSet<String>,
    ) {
        let expression = self.substitute_expression_bindings(expression, static_bindings);
        match &expression {
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.collect_static_parameter_member_write_property_values_from_expression_with_context(
                    object,
                    actual_target_expression,
                    target_name,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
                self.collect_static_parameter_member_write_property_values_from_expression_with_context(
                    property,
                    actual_target_expression,
                    target_name,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
                self.collect_static_parameter_member_write_property_values_from_expression_with_context(
                    value,
                    actual_target_expression,
                    target_name,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
                self.collect_static_parameter_member_assignment_value(
                    object,
                    property,
                    value,
                    actual_target_expression,
                    target_name,
                    current_function_name,
                    property_values,
                    visited_functions,
                );
            }
            Expression::Assign { name, value } => {
                self.collect_static_parameter_member_write_property_values_from_expression_with_context(
                    value,
                    actual_target_expression,
                    target_name,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
                self.update_static_function_alias(
                    name,
                    value,
                    current_function_name,
                    function_aliases,
                );
                Self::update_static_parameter_writeback_alias(
                    name,
                    value,
                    target_name,
                    static_bindings,
                );
            }
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                self.collect_static_parameter_member_write_property_values_from_expression_with_context(
                    callee,
                    actual_target_expression,
                    target_name,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
                for argument in arguments {
                    self.collect_static_parameter_member_write_property_values_from_expression_with_context(
                        argument.expression(),
                        actual_target_expression,
                        target_name,
                        current_function_name,
                        property_values,
                        function_aliases,
                        static_bindings,
                        visited_functions,
                    );
                }
                if let Some(function_name) = self.resolve_static_user_function_name_for_expression(
                    callee,
                    current_function_name,
                    function_aliases,
                ) && let Some(callee_declaration) =
                    self.resolve_registered_function_declaration(&function_name)
                {
                    for (index, argument) in arguments.iter().enumerate() {
                        let argument = self
                            .substitute_expression_bindings(argument.expression(), static_bindings);
                        if !Self::expression_is_static_parameter_writeback_target(
                            &argument,
                            target_name,
                        ) {
                            continue;
                        }
                        let Some(parameter) = callee_declaration.params.get(index) else {
                            continue;
                        };
                        self.collect_static_parameter_member_write_property_values_from_user_function_call(
                            &function_name,
                            &parameter.name,
                            actual_target_expression,
                            arguments,
                            property_values,
                            visited_functions,
                            static_bindings,
                        );
                    }
                }
            }
            Expression::AssignSuperMember { property, value } => {
                self.collect_static_parameter_member_write_property_values_from_expression_with_context(
                    property,
                    actual_target_expression,
                    target_name,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
                self.collect_static_parameter_member_write_property_values_from_expression_with_context(
                    value,
                    actual_target_expression,
                    target_name,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
            }
            Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => {
                self.collect_static_parameter_member_write_property_values_from_expression_with_context(
                    value,
                    actual_target_expression,
                    target_name,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
            }
            Expression::Member { object, property } => {
                self.collect_static_parameter_member_write_property_values_from_expression_with_context(
                    object,
                    actual_target_expression,
                    target_name,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
                self.collect_static_parameter_member_write_property_values_from_expression_with_context(
                    property,
                    actual_target_expression,
                    target_name,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
            }
            Expression::SuperMember { property } => {
                self.collect_static_parameter_member_write_property_values_from_expression_with_context(
                    property,
                    actual_target_expression,
                    target_name,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
            }
            Expression::Binary { left, right, .. } => {
                self.collect_static_parameter_member_write_property_values_from_expression_with_context(
                    left,
                    actual_target_expression,
                    target_name,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
                self.collect_static_parameter_member_write_property_values_from_expression_with_context(
                    right,
                    actual_target_expression,
                    target_name,
                    current_function_name,
                    property_values,
                    function_aliases,
                    static_bindings,
                    visited_functions,
                );
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                for expression in [condition, then_expression, else_expression] {
                    self.collect_static_parameter_member_write_property_values_from_expression_with_context(
                        expression,
                        actual_target_expression,
                        target_name,
                        current_function_name,
                        property_values,
                        function_aliases,
                        static_bindings,
                        visited_functions,
                    );
                }
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    self.collect_static_parameter_member_write_property_values_from_expression_with_context(
                        expression,
                        actual_target_expression,
                        target_name,
                        current_function_name,
                        property_values,
                        function_aliases,
                        static_bindings,
                        visited_functions,
                    );
                }
            }
            Expression::Array(elements) => {
                for element in elements {
                    match element {
                        ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                            self.collect_static_parameter_member_write_property_values_from_expression_with_context(
                                expression,
                                actual_target_expression,
                                target_name,
                                current_function_name,
                                property_values,
                                function_aliases,
                                static_bindings,
                                visited_functions,
                            );
                        }
                    }
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            for expression in [key, value] {
                                self.collect_static_parameter_member_write_property_values_from_expression_with_context(
                                    expression,
                                    actual_target_expression,
                                    target_name,
                                    current_function_name,
                                    property_values,
                                    function_aliases,
                                    static_bindings,
                                    visited_functions,
                                );
                            }
                        }
                        ObjectEntry::Getter { key, getter } => {
                            for expression in [key, getter] {
                                self.collect_static_parameter_member_write_property_values_from_expression_with_context(
                                    expression,
                                    actual_target_expression,
                                    target_name,
                                    current_function_name,
                                    property_values,
                                    function_aliases,
                                    static_bindings,
                                    visited_functions,
                                );
                            }
                        }
                        ObjectEntry::Setter { key, setter } => {
                            for expression in [key, setter] {
                                self.collect_static_parameter_member_write_property_values_from_expression_with_context(
                                    expression,
                                    actual_target_expression,
                                    target_name,
                                    current_function_name,
                                    property_values,
                                    function_aliases,
                                    static_bindings,
                                    visited_functions,
                                );
                            }
                        }
                        ObjectEntry::Spread(expression) => {
                            self.collect_static_parameter_member_write_property_values_from_expression_with_context(
                                expression,
                                actual_target_expression,
                                target_name,
                                current_function_name,
                                property_values,
                                function_aliases,
                                static_bindings,
                                visited_functions,
                            );
                        }
                    }
                }
            }
            Expression::Update { .. }
            | Expression::Identifier(_)
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::This
            | Expression::NewTarget
            | Expression::Sent => {}
        }
    }

    pub(in crate::backend::direct_wasm) fn user_function_static_argument_object_member_writeback_values(
        &self,
        user_function: &UserFunction,
        argument_expressions: &[Expression],
    ) -> Vec<(String, String, BTreeMap<String, Expression>)> {
        let Some(function) = self.resolve_registered_function_declaration(&user_function.name)
        else {
            return Vec::new();
        };
        let call_arguments = argument_expressions
            .iter()
            .cloned()
            .map(CallArgument::Expression)
            .collect::<Vec<_>>();
        let mut writebacks = Vec::new();
        for (index, parameter) in function.params.iter().enumerate() {
            let Some(argument_expression) = argument_expressions.get(index) else {
                continue;
            };
            let source_owner = match argument_expression {
                Expression::Identifier(name) => {
                    self.runtime_object_property_shadow_owner_name_for_identifier(name)
                }
                Expression::This => {
                    self.runtime_object_property_shadow_owner_name_for_identifier("this")
                }
                _ => None,
            };
            let Some(source_owner) = source_owner else {
                continue;
            };
            if source_owner == parameter.name {
                continue;
            }
            let mut property_values = BTreeMap::new();
            let mut visited_functions = HashSet::new();
            self.collect_static_parameter_member_write_property_values_from_user_function_call(
                &user_function.name,
                &parameter.name,
                argument_expression,
                &call_arguments,
                &mut property_values,
                &mut visited_functions,
                &HashMap::new(),
            );
            property_values.retain(|property_name, _| {
                self.static_argument_member_writeback_allowed(argument_expression, property_name)
            });
            if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                eprintln!(
                    "static_arg_member_writeback function={} param={} source_owner={} values={property_values:?}",
                    user_function.name, parameter.name, source_owner,
                );
            }
            if !property_values.is_empty() {
                writebacks.push((parameter.name.clone(), source_owner, property_values));
            }
        }
        writebacks
    }

    fn set_static_argument_object_member_writeback_property(
        object_binding: &mut ObjectValueBinding,
        property: Expression,
        value: Expression,
    ) {
        let canonical_property = static_property_name_from_expression(&property)
            .map(Expression::String)
            .unwrap_or_else(|| property.clone());
        object_binding_set_property(object_binding, property, value.clone());
        if let Some((_, descriptor)) = object_binding
            .property_descriptors
            .iter_mut()
            .find(|(existing_property, _)| *existing_property == canonical_property)
            && !descriptor.has_get
            && !descriptor.has_set
            && descriptor.getter.is_none()
            && descriptor.setter.is_none()
        {
            descriptor.value = Some(value);
        }
    }

    fn static_argument_member_writeback_allowed(
        &self,
        argument_expression: &Expression,
        property_name: &str,
    ) -> bool {
        if !property_name.starts_with("__ayy$private$") {
            return true;
        }

        let property = Expression::String(property_name.to_string());
        let object_binding = match argument_expression {
            Expression::Identifier(name) => self
                .resolve_runtime_shadow_object_binding(name)
                .or_else(|| self.resolve_object_binding_from_expression(argument_expression)),
            Expression::This => self
                .resolve_runtime_shadow_object_binding("this")
                .or_else(|| self.resolve_object_binding_from_expression(argument_expression)),
            _ => self.resolve_object_binding_from_expression(argument_expression),
        };
        let Some(object_binding) = object_binding else {
            return true;
        };

        object_binding_has_property(&object_binding, &property)
            || private_brand_marker_property_expression(&property)
                .is_some_and(|marker| object_binding_has_property(&object_binding, &marker))
    }

    pub(in crate::backend::direct_wasm) fn predeclare_static_argument_object_member_writeback_properties(
        &mut self,
        writebacks: &[(String, String, BTreeMap<String, Expression>)],
    ) {
        for (param_name, _, property_values) in writebacks {
            for property_name in property_values.keys() {
                if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                    eprintln!(
                        "static_arg_member_writeback_predeclare param={param_name} property={property_name}"
                    );
                }
                self.predeclare_runtime_shadow_property(param_name, property_name);
            }
        }
    }

    pub(in crate::backend::direct_wasm) fn sync_static_argument_object_member_writeback_values(
        &mut self,
        writebacks: &[(String, String, BTreeMap<String, Expression>)],
    ) {
        for (param_name, source_owner, property_values) in writebacks {
            let mut param_binding = self
                .resolve_runtime_shadow_object_binding(param_name)
                .unwrap_or_else(empty_object_value_binding);
            let mut source_binding = self
                .resolve_runtime_shadow_object_binding(source_owner)
                .or_else(|| {
                    self.resolve_object_binding_from_expression(&Expression::Identifier(
                        source_owner.clone(),
                    ))
                })
                .unwrap_or_else(empty_object_value_binding);
            for (property_name, value) in property_values {
                let property = Expression::String(property_name.clone());
                let value = self.reference_preserving_static_value_expression(value);
                if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                    eprintln!(
                        "static_arg_member_writeback_sync param={param_name} source_owner={source_owner} property={property_name} value={value:?}"
                    );
                }
                Self::set_static_argument_object_member_writeback_property(
                    &mut param_binding,
                    property.clone(),
                    value.clone(),
                );
                Self::set_static_argument_object_member_writeback_property(
                    &mut source_binding,
                    property,
                    value,
                );
            }
            self.sync_runtime_object_shadow_owner_static_metadata_from_binding(
                param_name,
                &param_binding,
            );
            self.sync_runtime_object_shadow_owner_static_metadata_from_binding(
                source_owner,
                &source_binding,
            );
        }
    }

    fn emit_global_this_shadow_commit_for_property_names(
        &mut self,
        property_values: &BTreeMap<String, Expression>,
    ) -> DirectResult<()> {
        let updated_this_binding = self.resolve_runtime_shadow_object_binding("this");
        for (property_name, materialized_value) in property_values {
            let property = Expression::String(property_name.clone());
            let shadow_binding =
                self.runtime_object_property_shadow_binding_by_names("this", property_name);
            let deleted_binding =
                self.runtime_object_property_shadow_deleted_binding_by_property("this", &property);
            let target_binding = self.ensure_implicit_global_binding(property_name);

            self.push_global_get(deleted_binding.present_index);
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.push_i32_const(0);
            self.push_global_set(target_binding.present_index);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(target_binding.value_index);
            self.state.emission.output.instructions.push(0x05);
            self.push_global_get(shadow_binding.present_index);
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.push_global_get(shadow_binding.value_index);
            self.push_global_set(target_binding.value_index);
            self.push_i32_const(1);
            self.push_global_set(target_binding.present_index);
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();

            self.update_static_global_assignment_metadata(property_name, materialized_value);
            self.update_global_property_descriptor_value(property_name, materialized_value);
        }

        if let Some(updated_this_binding) = updated_this_binding {
            let updated_this_expression = object_binding_to_expression(&updated_this_binding);
            self.update_local_value_binding("this", &updated_this_expression);
            self.update_local_object_binding("this", &updated_this_expression);
        }

        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn prepare_user_function_runtime_this_shadow_state(
        &mut self,
        this_expression: &Expression,
    ) -> DirectResult<Option<String>> {
        let target_owner = self.resolve_user_function_call_receiver_shadow_owner(this_expression);
        if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
            eprintln!(
                "runtime_this_shadow_prepare fn={:?} this_expression={this_expression:?} target_owner={target_owner:?}",
                self.current_function_name(),
            );
        }
        let receiver_already_materialized_in_this =
            target_owner.is_none() && matches!(this_expression, Expression::New { .. });
        let saved_shadow_owner = (target_owner.as_deref() != Some("this")
            && !receiver_already_materialized_in_this)
            .then(|| {
                self.allocate_named_hidden_local("saved_this_shadow", StaticValueKind::Object)
            });

        if let Some(saved_shadow_owner) = saved_shadow_owner.as_deref() {
            self.emit_runtime_object_property_shadow_copy("this", saved_shadow_owner)?;
            self.clear_runtime_object_property_shadow_prefix("this");
        }

        if let Some(target_owner) = target_owner.as_deref().filter(|owner| *owner != "this") {
            self.emit_runtime_object_property_shadow_copy(target_owner, "this")?;
        } else if target_owner.is_none()
            && !receiver_already_materialized_in_this
            && let Some(object_binding) =
                self.resolve_object_binding_from_expression(this_expression)
        {
            self.emit_runtime_object_property_shadow_seed_from_binding("this", &object_binding)?;
        }

        Ok(saved_shadow_owner)
    }

    pub(in crate::backend::direct_wasm) fn finalize_user_function_runtime_this_shadow_state(
        &mut self,
        user_function: &UserFunction,
        this_expression: &Expression,
        updated_bindings: Option<&HashMap<String, Expression>>,
        saved_shadow_owner: Option<&str>,
        allow_static_receiver_update: bool,
        receiver_updated_via_parameter_writeback: bool,
        receiver_may_require_invalidation: bool,
        argument_expressions: &[Expression],
    ) -> DirectResult<()> {
        let target_owner = self.resolve_user_function_call_receiver_shadow_owner(this_expression);
        let explicit_updated_this = updated_bindings.and_then(|bindings| bindings.get("this"));
        let receiver_is_module_namespace = self
            .resolve_object_binding_from_expression(this_expression)
            .is_some_and(|binding| Self::object_binding_has_module_namespace_marker(&binding));
        if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
            eprintln!(
                "runtime_this_shadow_finalize fn={:?} this_expression={this_expression:?} target_owner={target_owner:?} updated_this={:?} allow_static_receiver_update={} receiver_updated_via_parameter_writeback={} receiver_is_module_namespace={}",
                self.current_function_name(),
                explicit_updated_this,
                allow_static_receiver_update,
                receiver_updated_via_parameter_writeback,
                receiver_is_module_namespace,
            );
        }

        if let Some(target_owner) = target_owner.as_deref().filter(|owner| *owner != "this") {
            let property_values = if allow_static_receiver_update {
                self.user_function_static_this_member_write_property_values_from_arguments(
                    user_function,
                    argument_expressions,
                )
            } else {
                BTreeMap::new()
            };
            let mut property_names =
                self.user_function_static_this_member_write_property_names(user_function);
            property_names.extend(property_values.keys().cloned());
            for property_name in property_names {
                self.predeclare_runtime_shadow_property("this", &property_name);
            }
            self.sync_runtime_this_shadow_static_property_values(&property_values);
            let updated_receiver_binding = self.resolve_runtime_shadow_object_binding("this");
            let should_copy_runtime_this_shadow = !receiver_is_module_namespace
                && (explicit_updated_this.is_some() || !receiver_updated_via_parameter_writeback);
            if should_copy_runtime_this_shadow {
                self.emit_runtime_object_property_shadow_copy("this", target_owner)?;
            }
            if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                eprintln!(
                    "runtime_this_shadow_sync fn={:?} target_owner={target_owner} updated_receiver_binding_present={} copied={should_copy_runtime_this_shadow}",
                    self.current_function_name(),
                    updated_receiver_binding.is_some(),
                );
            }
            if let Some(updated_receiver_binding) = updated_receiver_binding {
                if should_copy_runtime_this_shadow {
                    self.sync_receiver_metadata_from_runtime_shadow(
                        this_expression,
                        target_owner,
                        &updated_receiver_binding,
                    );
                }
                if allow_static_receiver_update && explicit_updated_this.is_some() {
                    if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some()
                        && let Expression::Identifier(name) = this_expression
                    {
                        let updated_receiver_expression =
                            object_binding_to_expression(&updated_receiver_binding);
                        eprintln!(
                            "runtime_this_shadow_global_update name={name} updated_receiver_expression={updated_receiver_expression:?} direct_resolved_object={:?} value={:?} resolved_object={:?}",
                            self.resolve_object_binding_from_expression(
                                &updated_receiver_expression,
                            )
                            .map(|binding| object_binding_to_expression(&binding)),
                            self.global_value_binding(name).cloned(),
                            self.resolve_object_binding_from_expression(&Expression::Identifier(
                                name.to_string(),
                            ))
                            .map(|binding| object_binding_to_expression(&binding)),
                        );
                    }
                } else if !target_owner.contains("setter_receiver")
                    && !receiver_is_module_namespace
                    && !allow_static_receiver_update
                    && !receiver_updated_via_parameter_writeback
                    && receiver_may_require_invalidation
                {
                    self.clear_runtime_object_property_non_private_shadow_prefix(target_owner);
                    match this_expression {
                        Expression::Identifier(name)
                            if self.binding_name_is_global(name)
                                || self.global_has_binding(name)
                                || self.global_has_implicit_binding(name) =>
                        {
                            self.backend.clear_global_static_binding_metadata(name);
                            self.state.clear_local_static_binding_metadata(name);
                        }
                        Expression::Identifier(name) => {
                            self.state.clear_local_static_binding_metadata(name);
                        }
                        Expression::This => {
                            self.state.clear_local_static_binding_metadata("this");
                        }
                        _ => {}
                    }
                }
            }
        } else if target_owner.as_deref() == Some("this")
            && receiver_may_require_invalidation
            && self.state.speculation.execution_context.top_level_function
        {
            let property_values = self
                .user_function_static_this_member_write_property_values_from_arguments(
                    user_function,
                    argument_expressions,
                );
            if !property_values.is_empty() {
                if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                    let property_names = property_values.keys().collect::<Vec<_>>();
                    eprintln!("runtime_this_shadow_global_commit properties={property_names:?}");
                }
                self.emit_global_this_shadow_commit_for_property_names(&property_values)?;
            }
        }

        if let Some(saved_shadow_owner) = saved_shadow_owner {
            self.clear_runtime_object_property_shadow_prefix("this");
            self.emit_runtime_object_property_shadow_copy(saved_shadow_owner, "this")?;
        } else if target_owner.as_deref() != Some("this") {
            self.clear_runtime_object_property_shadow_prefix("this");
        }

        if allow_static_receiver_update && let Some(updated_this) = explicit_updated_this {
            match this_expression {
                Expression::Identifier(name) => {
                    self.update_local_value_binding(name, updated_this);
                    self.update_local_object_binding(name, updated_this);
                }
                Expression::This => {
                    self.update_local_value_binding("this", updated_this);
                    self.update_local_object_binding("this", updated_this);
                }
                _ => {}
            }
        }

        Ok(())
    }
}
