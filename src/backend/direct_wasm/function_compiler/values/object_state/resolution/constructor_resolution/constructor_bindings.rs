use super::*;

impl<'a> FunctionCompiler<'a> {
    fn push_constructor_super_owner_candidate(candidates: &mut Vec<String>, name: String) {
        if !candidates.iter().any(|candidate| candidate == &name) {
            candidates.push(name);
        }
    }

    fn constructor_super_owner_candidates(&self, owner_name: &str) -> Vec<String> {
        let mut candidates = vec![owner_name.to_string()];
        let mut index = 0;
        while index < candidates.len() && candidates.len() < 12 {
            let candidate = candidates[index].clone();
            let candidate_expression = Expression::Identifier(candidate.clone());
            if let Some(Expression::Identifier(resolved_name)) = self
                .resolve_bound_alias_expression(&candidate_expression)
                .filter(|resolved| !static_expression_matches(resolved, &candidate_expression))
            {
                Self::push_constructor_super_owner_candidate(&mut candidates, resolved_name);
            }
            if let Some(Expression::Identifier(value_name)) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(&candidate)
                .or_else(|| self.global_value_binding(&candidate))
            {
                Self::push_constructor_super_owner_candidate(&mut candidates, value_name.clone());
            }
            if let Some(binding) = self.resolve_function_binding_from_expression(
                &Expression::Identifier(candidate.clone()),
            ) {
                if let Some(owner_name) = self.function_prototype_binding_owner_name(&binding) {
                    Self::push_constructor_super_owner_candidate(&mut candidates, owner_name);
                }
                if let LocalFunctionBinding::User(function_name) = binding
                    && let Some(self_binding) = self
                        .resolve_registered_function_declaration(&function_name)
                        .and_then(|function| function.self_binding.clone())
                {
                    Self::push_constructor_super_owner_candidate(&mut candidates, self_binding);
                }
            }
            index += 1;
        }
        candidates
    }

    fn normalize_constructor_super_source_expression(&self, expression: Expression) -> Expression {
        let expression = match expression {
            Expression::Member { object, property } if matches!(property.as_ref(), Expression::String(name) if name == "prototype") => {
                *object
            }
            other => other,
        };
        let resolved = self
            .resolve_bound_alias_expression(&expression)
            .filter(|resolved| !static_expression_matches(resolved, &expression))
            .unwrap_or(expression);
        match resolved {
            Expression::Member { object, property } if matches!(property.as_ref(), Expression::String(name) if name == "prototype") => {
                *object
            }
            other => other,
        }
    }

    fn resolve_constructor_synthetic_super_source_expression(
        &self,
        owner_name: &str,
    ) -> Option<Expression> {
        for candidate in self.constructor_super_owner_candidates(owner_name) {
            let Some(prototype) = self.global_object_prototype_expression(&candidate).cloned()
            else {
                continue;
            };
            let super_source = self.normalize_constructor_super_source_expression(prototype);
            if matches!(
                &super_source,
                Expression::Identifier(name) if name == owner_name || name == &candidate
            ) {
                continue;
            }
            return Some(super_source);
        }
        None
    }

    fn static_constructor_statement_initializes_function_statement_binding(
        statement: &Statement,
        target_name: &str,
    ) -> bool {
        match statement {
            Statement::Let {
                name,
                mutable: true,
                value: Expression::Identifier(function_name),
            } => name == target_name && function_name.starts_with("__ayy_fnstmt_"),
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => body.iter().any(|statement| {
                Self::static_constructor_statement_initializes_function_statement_binding(
                    statement,
                    target_name,
                )
            }),
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => {
                then_branch.iter().any(|statement| {
                    Self::static_constructor_statement_initializes_function_statement_binding(
                        statement,
                        target_name,
                    )
                }) || else_branch.iter().any(|statement| {
                    Self::static_constructor_statement_initializes_function_statement_binding(
                        statement,
                        target_name,
                    )
                })
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => body
                .iter()
                .chain(catch_setup)
                .chain(catch_body)
                .any(|statement| {
                    Self::static_constructor_statement_initializes_function_statement_binding(
                        statement,
                        target_name,
                    )
                }),
            Statement::For { init, body, .. } => init.iter().chain(body).any(|statement| {
                Self::static_constructor_statement_initializes_function_statement_binding(
                    statement,
                    target_name,
                )
            }),
            Statement::While { body, .. } | Statement::DoWhile { body, .. } => {
                body.iter().any(|statement| {
                    Self::static_constructor_statement_initializes_function_statement_binding(
                        statement,
                        target_name,
                    )
                })
            }
            Statement::Switch { cases, .. } => cases.iter().any(|case| {
                case.body.iter().any(|statement| {
                    Self::static_constructor_statement_initializes_function_statement_binding(
                        statement,
                        target_name,
                    )
                })
            }),
            _ => false,
        }
    }

    fn static_constructor_expression_assigns_this_member_property(
        expression: &Expression,
        target_name: &str,
    ) -> bool {
        match expression {
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                let assigns_target = matches!(
                    (object.as_ref(), property.as_ref()),
                    (Expression::This, Expression::String(name)) if name == target_name
                ) || matches!(
                    (object.as_ref(), property.as_ref()),
                    (
                        Expression::Identifier(object_name),
                        Expression::String(property_name),
                    ) if object_name == Self::STATIC_NEW_THIS_BINDING
                        && property_name == target_name
                );
                assigns_target
                    || Self::static_constructor_expression_assigns_this_member_property(
                        value,
                        target_name,
                    )
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => {
                Self::static_constructor_expression_assigns_this_member_property(value, target_name)
            }
            Expression::Member { object, property } => {
                Self::static_constructor_expression_assigns_this_member_property(
                    object,
                    target_name,
                ) || Self::static_constructor_expression_assigns_this_member_property(
                    property,
                    target_name,
                )
            }
            Expression::SuperCall { callee, arguments } | Expression::New { callee, arguments } => {
                Self::static_constructor_expression_assigns_this_member_property(
                    callee,
                    target_name,
                ) || arguments.iter().any(|argument| match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        Self::static_constructor_expression_assigns_this_member_property(
                            expression,
                            target_name,
                        )
                    }
                })
            }
            Expression::Call { callee, arguments } => {
                Self::static_constructor_expression_assigns_this_member_property(
                    callee,
                    target_name,
                ) || arguments.iter().any(|argument| match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        Self::static_constructor_expression_assigns_this_member_property(
                            expression,
                            target_name,
                        )
                    }
                })
            }
            Expression::SuperMember { property } => {
                Self::static_constructor_expression_assigns_this_member_property(
                    property,
                    target_name,
                )
            }
            Expression::AssignSuperMember { property, value } => {
                Self::static_constructor_expression_assigns_this_member_property(
                    property,
                    target_name,
                ) || Self::static_constructor_expression_assigns_this_member_property(
                    value,
                    target_name,
                )
            }
            Expression::Binary { left, right, .. } => {
                Self::static_constructor_expression_assigns_this_member_property(left, target_name)
                    || Self::static_constructor_expression_assigns_this_member_property(
                        right,
                        target_name,
                    )
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::static_constructor_expression_assigns_this_member_property(
                    condition,
                    target_name,
                ) || Self::static_constructor_expression_assigns_this_member_property(
                    then_expression,
                    target_name,
                ) || Self::static_constructor_expression_assigns_this_member_property(
                    else_expression,
                    target_name,
                )
            }
            Expression::Sequence(expressions) => expressions.iter().any(|expression| {
                Self::static_constructor_expression_assigns_this_member_property(
                    expression,
                    target_name,
                )
            }),
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                    Self::static_constructor_expression_assigns_this_member_property(
                        expression,
                        target_name,
                    )
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    Self::static_constructor_expression_assigns_this_member_property(
                        key,
                        target_name,
                    ) || Self::static_constructor_expression_assigns_this_member_property(
                        value,
                        target_name,
                    )
                }
                ObjectEntry::Getter { key, getter } => {
                    Self::static_constructor_expression_assigns_this_member_property(
                        key,
                        target_name,
                    ) || Self::static_constructor_expression_assigns_this_member_property(
                        getter,
                        target_name,
                    )
                }
                ObjectEntry::Setter { key, setter } => {
                    Self::static_constructor_expression_assigns_this_member_property(
                        key,
                        target_name,
                    ) || Self::static_constructor_expression_assigns_this_member_property(
                        setter,
                        target_name,
                    )
                }
                ObjectEntry::Spread(expression) => {
                    Self::static_constructor_expression_assigns_this_member_property(
                        expression,
                        target_name,
                    )
                }
            }),
            Expression::Identifier(_)
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::This
            | Expression::NewTarget
            | Expression::Sent
            | Expression::Update { .. } => false,
        }
    }

    fn static_constructor_statement_assigns_this_member_property(
        statement: &Statement,
        target_name: &str,
    ) -> bool {
        match statement {
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                let assigns_target = matches!(
                    (object, property),
                    (Expression::This, Expression::String(name)) if name == target_name
                ) || matches!(
                    (object, property),
                    (Expression::Identifier(object_name), Expression::String(property_name))
                        if object_name == Self::STATIC_NEW_THIS_BINDING
                            && property_name == target_name
                );
                assigns_target
                    || Self::static_constructor_expression_assigns_this_member_property(
                        value,
                        target_name,
                    )
            }
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => body.iter().any(|statement| {
                Self::static_constructor_statement_assigns_this_member_property(
                    statement,
                    target_name,
                )
            }),
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                Self::static_constructor_expression_assigns_this_member_property(
                    condition,
                    target_name,
                ) || then_branch.iter().any(|statement| {
                    Self::static_constructor_statement_assigns_this_member_property(
                        statement,
                        target_name,
                    )
                }) || else_branch.iter().any(|statement| {
                    Self::static_constructor_statement_assigns_this_member_property(
                        statement,
                        target_name,
                    )
                })
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => body
                .iter()
                .chain(catch_setup)
                .chain(catch_body)
                .any(|statement| {
                    Self::static_constructor_statement_assigns_this_member_property(
                        statement,
                        target_name,
                    )
                }),
            Statement::For { init, body, .. } => init.iter().chain(body).any(|statement| {
                Self::static_constructor_statement_assigns_this_member_property(
                    statement,
                    target_name,
                )
            }),
            Statement::While {
                body, condition, ..
            }
            | Statement::DoWhile {
                body, condition, ..
            } => {
                Self::static_constructor_expression_assigns_this_member_property(
                    condition,
                    target_name,
                ) || body.iter().any(|statement| {
                    Self::static_constructor_statement_assigns_this_member_property(
                        statement,
                        target_name,
                    )
                })
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                Self::static_constructor_expression_assigns_this_member_property(
                    discriminant,
                    target_name,
                ) || cases.iter().any(|case| {
                    case.body.iter().any(|statement| {
                        Self::static_constructor_statement_assigns_this_member_property(
                            statement,
                            target_name,
                        )
                    })
                })
            }
            Statement::Var { value, .. }
            | Statement::Let { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Expression(value)
            | Statement::Throw(value)
            | Statement::Return(value)
            | Statement::Yield { value }
            | Statement::YieldDelegate { value } => {
                Self::static_constructor_expression_assigns_this_member_property(value, target_name)
            }
            Statement::Print { values } => values.iter().any(|value| {
                Self::static_constructor_expression_assigns_this_member_property(value, target_name)
            }),
            Statement::With { object, body } => {
                Self::static_constructor_expression_assigns_this_member_property(
                    object,
                    target_name,
                ) || body.iter().any(|statement| {
                    Self::static_constructor_statement_assigns_this_member_property(
                        statement,
                        target_name,
                    )
                })
            }
            Statement::Break { .. } | Statement::Continue { .. } => false,
        }
    }

    fn static_constructor_snapshot_blocking_assigned_nonlocal_bindings(
        statements: &[Statement],
        assigned_nonlocal_bindings: &HashSet<String>,
    ) -> Vec<String> {
        assigned_nonlocal_bindings
            .iter()
            .filter(|name| {
                !statements.iter().any(|statement| {
                    Self::static_constructor_statement_initializes_function_statement_binding(
                        statement, name,
                    )
                }) && !statements.iter().any(|statement| {
                    Self::static_constructor_statement_assigns_this_member_property(statement, name)
                })
            })
            .cloned()
            .collect()
    }

    fn static_constructor_return_assignment_alias(statements: &[Statement]) -> Option<String> {
        for statement in statements {
            match statement {
                Statement::Declaration { body }
                | Statement::Block { body }
                | Statement::Labeled { body, .. } => {
                    if let Some(alias) = Self::static_constructor_return_assignment_alias(body) {
                        return Some(alias);
                    }
                }
                Statement::If {
                    then_branch,
                    else_branch,
                    ..
                } => {
                    let then_alias = Self::static_constructor_return_assignment_alias(then_branch);
                    let else_alias = Self::static_constructor_return_assignment_alias(else_branch);
                    if then_alias.is_some() && then_alias == else_alias {
                        return then_alias;
                    }
                }
                Statement::Return(expression) => {
                    return Self::static_constructor_return_expression_assignment_alias(expression);
                }
                _ => {}
            }
        }
        None
    }

    fn static_constructor_return_expression_assignment_alias(
        expression: &Expression,
    ) -> Option<String> {
        match expression {
            Expression::Assign { name, .. } => Some(name.clone()),
            Expression::Sequence(expressions) => expressions
                .last()
                .and_then(Self::static_constructor_return_expression_assignment_alias),
            Expression::Conditional {
                then_expression,
                else_expression,
                ..
            } => {
                let then_alias =
                    Self::static_constructor_return_expression_assignment_alias(then_expression);
                let else_alias =
                    Self::static_constructor_return_expression_assignment_alias(else_expression);
                (then_alias == else_alias).then_some(then_alias).flatten()
            }
            _ => None,
        }
    }

    fn static_constructor_return_value_matches_alias(
        &self,
        alias_value: &Expression,
        return_value: &Expression,
        environment: &mut StaticResolutionEnvironment,
    ) -> bool {
        if static_expression_matches(alias_value, return_value) {
            return true;
        }
        let alias_binding =
            self.resolve_object_binding_from_expression_with_state(alias_value, environment);
        let return_binding =
            self.resolve_object_binding_from_expression_with_state(return_value, environment);
        alias_binding.is_some() && alias_binding == return_binding
    }

    fn seed_constructed_private_member_markers(
        &self,
        constructor_function_name: &str,
        object_binding: &mut ObjectValueBinding,
    ) {
        let Some(class_name) = self
            .resolve_registered_function_declaration(constructor_function_name)
            .and_then(|function| function.self_binding.as_deref())
            .map(str::to_string)
            .or_else(|| {
                constructor_function_name
                    .rsplit_once("__name_")
                    .map(|(_, class_name)| class_name.to_string())
            })
        else {
            return;
        };
        let metadata_targets = [constructor_function_name, class_name.as_str()];
        let private_member_binding_expression = |binding: &LocalFunctionBinding| match binding {
            LocalFunctionBinding::User(function_name)
            | LocalFunctionBinding::Builtin(function_name) => {
                Expression::Identifier(function_name.clone())
            }
        };
        let private_member_marker_expression = |binding: &LocalFunctionBinding| match binding {
            LocalFunctionBinding::User(function_name) => self
                .user_function(function_name)
                .and_then(|function| function.private_brand_binding.as_ref())
                .map(|binding_name| Expression::Identifier(binding_name.clone()))
                .unwrap_or_else(|| Expression::Identifier(function_name.clone())),
            LocalFunctionBinding::Builtin(function_name) => {
                Expression::Identifier(function_name.clone())
            }
        };
        let private_member_marker_property = |property_name: &str| {
            private_brand_marker_property_expression(&Expression::String(property_name.to_string()))
        };
        let trace_private = std::env::var_os("AYY_TRACE_PRIVATE_MEMBER_LOOKUP").is_some();
        if trace_private {
            eprintln!(
                "private_seed_function constructor={} class={}",
                constructor_function_name, class_name
            );
        }

        let prototype_binding = self
            .resolve_function_prototype_object_binding(constructor_function_name)
            .or_else(|| self.resolve_function_prototype_object_binding(&class_name));
        if trace_private {
            eprintln!(
                "private_seed_function prototype_props={:?}",
                prototype_binding
                    .as_ref()
                    .map(ordered_object_property_names)
                    .unwrap_or_default(),
            );
        }
        if let Some(prototype_binding) = prototype_binding {
            for (property_name, value) in &prototype_binding.string_properties {
                if !property_name.starts_with("__ayy$private$") {
                    continue;
                }
                let enumerable = !prototype_binding
                    .non_enumerable_string_properties
                    .iter()
                    .any(|hidden_name| hidden_name == property_name);
                object_binding_define_property(
                    object_binding,
                    Expression::String(property_name.clone()),
                    value.clone(),
                    enumerable,
                );
                if let Some(marker_property) = private_member_marker_property(property_name) {
                    object_binding_define_property(
                        object_binding,
                        marker_property.clone(),
                        value.clone(),
                        false,
                    );
                    if trace_private {
                        eprintln!("private_seed_function prototype marker={marker_property:?}");
                    }
                }
                if trace_private {
                    eprintln!("private_seed_function prototype property={property_name}");
                }
            }
        }

        for (key, binding) in &self.backend.global_semantics.members.member_getter_bindings {
            let MemberFunctionBindingTarget::Prototype(target_name) = &key.target else {
                continue;
            };
            let MemberFunctionBindingProperty::String(property_name) = &key.property else {
                continue;
            };
            if !metadata_targets
                .iter()
                .any(|candidate| target_name == candidate)
                || !property_name.starts_with("__ayy$private$")
            {
                continue;
            }
            let member_value = private_member_binding_expression(binding);
            let marker_value = private_member_marker_expression(binding);
            object_binding_define_property(
                object_binding,
                Expression::String(property_name.clone()),
                member_value,
                false,
            );
            if let Some(marker_property) = private_member_marker_property(property_name) {
                object_binding_define_property(
                    object_binding,
                    marker_property.clone(),
                    marker_value,
                    false,
                );
                if trace_private {
                    eprintln!("private_seed_function getter marker={marker_property:?}");
                }
            }
            if trace_private {
                eprintln!("private_seed_function getter property={property_name}");
            }
        }

        for (key, binding) in &self
            .backend
            .global_semantics
            .members
            .member_function_bindings
        {
            let MemberFunctionBindingTarget::Prototype(target_name) = &key.target else {
                continue;
            };
            let MemberFunctionBindingProperty::String(property_name) = &key.property else {
                continue;
            };
            if !metadata_targets
                .iter()
                .any(|candidate| target_name == candidate)
                || !property_name.starts_with("__ayy$private$")
            {
                continue;
            }
            let member_value = private_member_binding_expression(binding);
            let marker_value = private_member_marker_expression(binding);
            object_binding_define_property(
                object_binding,
                Expression::String(property_name.clone()),
                member_value,
                false,
            );
            if let Some(marker_property) = private_member_marker_property(property_name) {
                object_binding_define_property(
                    object_binding,
                    marker_property.clone(),
                    marker_value,
                    false,
                );
                if trace_private {
                    eprintln!("private_seed_function method marker={marker_property:?}");
                }
            }
            if trace_private {
                eprintln!("private_seed_function method property={property_name}");
            }
        }

        for (key, binding) in &self.backend.global_semantics.members.member_setter_bindings {
            let MemberFunctionBindingTarget::Prototype(target_name) = &key.target else {
                continue;
            };
            let MemberFunctionBindingProperty::String(property_name) = &key.property else {
                continue;
            };
            if !metadata_targets
                .iter()
                .any(|candidate| target_name == candidate)
                || !property_name.starts_with("__ayy$private$")
            {
                continue;
            }
            let member_value = private_member_binding_expression(binding);
            let marker_value = private_member_marker_expression(binding);
            object_binding_define_property(
                object_binding,
                Expression::String(property_name.clone()),
                member_value,
                false,
            );
            if let Some(marker_property) = private_member_marker_property(property_name) {
                object_binding_define_property(
                    object_binding,
                    marker_property.clone(),
                    marker_value,
                    false,
                );
                if trace_private {
                    eprintln!("private_seed_function setter marker={marker_property:?}");
                }
            }
            if trace_private {
                eprintln!("private_seed_function setter property={property_name}");
            }
        }
    }

    fn resolve_static_constructor_binding_value(
        &self,
        expression: &Expression,
        environment: &mut StaticResolutionEnvironment,
    ) -> Expression {
        self.evaluate_static_expression_with_state(expression, environment)
            .or_else(|| self.materialize_static_expression_with_state(expression, environment))
            .unwrap_or_else(|| expression.clone())
    }

    fn substitute_static_constructor_capture_source_expression(
        expression: &Expression,
        capture_source_bindings: &HashMap<String, Expression>,
    ) -> Expression {
        if let Expression::Identifier(name) = expression
            && let Some(source_expression) = capture_source_bindings.get(name)
        {
            return source_expression.clone();
        }
        materialize_recursive_expression(expression, true, true, &|nested| {
            Some(
                Self::substitute_static_constructor_capture_source_expression(
                    nested,
                    capture_source_bindings,
                ),
            )
        })
        .unwrap_or_else(|| expression.clone())
    }

    fn substitute_static_constructor_capture_sources_in_descriptor(
        descriptor: &mut PropertyDescriptorBinding,
        capture_source_bindings: &HashMap<String, Expression>,
    ) {
        if let Some(value) = descriptor.value.as_mut() {
            *value = Self::substitute_static_constructor_capture_source_expression(
                value,
                capture_source_bindings,
            );
        }
        if let Some(getter) = descriptor.getter.as_mut() {
            *getter = Self::substitute_static_constructor_capture_source_expression(
                getter,
                capture_source_bindings,
            );
        }
        if let Some(setter) = descriptor.setter.as_mut() {
            *setter = Self::substitute_static_constructor_capture_source_expression(
                setter,
                capture_source_bindings,
            );
        }
    }

    fn static_constructor_object_binding_with_capture_sources(
        mut object_binding: ObjectValueBinding,
        capture_source_bindings: Option<&HashMap<String, Expression>>,
    ) -> ObjectValueBinding {
        let Some(capture_source_bindings) = capture_source_bindings else {
            return object_binding;
        };
        for (_, value) in &mut object_binding.string_properties {
            *value = Self::substitute_static_constructor_capture_source_expression(
                value,
                capture_source_bindings,
            );
        }
        for (property, value) in &mut object_binding.symbol_properties {
            *property = Self::substitute_static_constructor_capture_source_expression(
                property,
                capture_source_bindings,
            );
            *value = Self::substitute_static_constructor_capture_source_expression(
                value,
                capture_source_bindings,
            );
        }
        for (property, descriptor) in &mut object_binding.property_descriptors {
            *property = Self::substitute_static_constructor_capture_source_expression(
                property,
                capture_source_bindings,
            );
            Self::substitute_static_constructor_capture_sources_in_descriptor(
                descriptor,
                capture_source_bindings,
            );
        }
        object_binding
    }

    pub(in crate::backend::direct_wasm) fn constructor_capture_source_is_stable_snapshot(
        name: &str,
    ) -> bool {
        name.starts_with("__ayy_class_field_name_")
            || name.starts_with("__ayy_class_super_")
            || name.starts_with("__ayy_class_brand_")
    }

    fn constructor_capture_sources_include_live_bindings(
        capture_source_bindings: Option<&HashMap<String, Expression>>,
    ) -> bool {
        capture_source_bindings.is_some_and(|bindings| {
            bindings
                .keys()
                .any(|name| !Self::constructor_capture_source_is_stable_snapshot(name))
        })
    }

    fn current_constructor_capture_source_expression(
        &self,
        source_name: &str,
        constructor_slot_owner: Option<&String>,
    ) -> Expression {
        if source_name == "this" {
            return Expression::This;
        }
        if source_name == "new.target" {
            return Expression::NewTarget;
        }
        if source_name.starts_with("__ayy_class_super_")
            && let Some(source) = constructor_slot_owner
                .and_then(|owner| self.resolve_constructor_synthetic_super_source_expression(owner))
        {
            return source;
        }

        let source_expression = Expression::Identifier(source_name.to_string());
        if self.state.speculation.execution_context.top_level_function
            && let Some(value) = self.global_value_binding(source_name).cloned()
        {
            return value;
        }
        self.state
            .speculation
            .static_semantics
            .local_value_binding(source_name)
            .cloned()
            .or_else(|| self.global_value_binding(source_name).cloned())
            .or_else(|| {
                self.resolve_bound_alias_expression(&source_expression)
                    .filter(|resolved| !static_expression_matches(resolved, &source_expression))
            })
            .unwrap_or(source_expression)
    }

    fn expression_is_static_constructor_this_reference(
        expression: &Expression,
        this_name: &str,
    ) -> bool {
        matches!(expression, Expression::This)
            || matches!(expression, Expression::Identifier(name) if name == this_name)
    }

    fn expand_static_constructor_this_references_in_object_binding(
        object_binding: &mut ObjectValueBinding,
        this_name: &str,
        this_object_binding: &ObjectValueBinding,
    ) {
        let this_expression = object_binding_to_expression(this_object_binding);
        for (_, value) in &mut object_binding.string_properties {
            if Self::expression_is_static_constructor_this_reference(value, this_name) {
                *value = this_expression.clone();
            }
        }
        for (_, value) in &mut object_binding.symbol_properties {
            if Self::expression_is_static_constructor_this_reference(value, this_name) {
                *value = this_expression.clone();
            }
        }
        for (_, descriptor) in &mut object_binding.property_descriptors {
            if let Some(value) = &mut descriptor.value
                && Self::expression_is_static_constructor_this_reference(value, this_name)
            {
                *value = this_expression.clone();
            }
        }
    }

    pub(in crate::backend::direct_wasm) fn expression_contains_static_update(
        expression: &Expression,
    ) -> bool {
        match expression {
            Expression::Update { .. } => true,
            Expression::Unary { expression, .. }
            | Expression::Await(expression)
            | Expression::EnumerateKeys(expression)
            | Expression::GetIterator(expression)
            | Expression::IteratorClose(expression) => {
                Self::expression_contains_static_update(expression)
            }
            Expression::Binary { left, right, .. } => {
                Self::expression_contains_static_update(left)
                    || Self::expression_contains_static_update(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::expression_contains_static_update(condition)
                    || Self::expression_contains_static_update(then_expression)
                    || Self::expression_contains_static_update(else_expression)
            }
            Expression::Sequence(expressions) => expressions
                .iter()
                .any(Self::expression_contains_static_update),
            Expression::Assign { value, .. } => Self::expression_contains_static_update(value),
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                Self::expression_contains_static_update(object)
                    || Self::expression_contains_static_update(property)
                    || Self::expression_contains_static_update(value)
            }
            Expression::AssignSuperMember { property, value } => {
                Self::expression_contains_static_update(property)
                    || Self::expression_contains_static_update(value)
            }
            Expression::Member { object, property } => {
                Self::expression_contains_static_update(object)
                    || Self::expression_contains_static_update(property)
            }
            Expression::SuperMember { property } => {
                Self::expression_contains_static_update(property)
            }
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                Self::expression_contains_static_update(callee)
                    || arguments.iter().any(|argument| match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            Self::expression_contains_static_update(expression)
                        }
                    })
            }
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                    Self::expression_contains_static_update(expression)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    Self::expression_contains_static_update(key)
                        || Self::expression_contains_static_update(value)
                }
                ObjectEntry::Getter { key, getter } => {
                    Self::expression_contains_static_update(key)
                        || Self::expression_contains_static_update(getter)
                }
                ObjectEntry::Setter { key, setter } => {
                    Self::expression_contains_static_update(key)
                        || Self::expression_contains_static_update(setter)
                }
                ObjectEntry::Spread(expression) => {
                    Self::expression_contains_static_update(expression)
                }
            }),
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::Identifier(_)
            | Expression::This
            | Expression::NewTarget
            | Expression::Sent => false,
        }
    }

    pub(in crate::backend::direct_wasm) fn object_binding_contains_static_update(
        object_binding: &ObjectValueBinding,
    ) -> bool {
        object_binding
            .string_properties
            .iter()
            .any(|(_, value)| Self::expression_contains_static_update(value))
            || object_binding
                .symbol_properties
                .iter()
                .any(|(property, value)| {
                    Self::expression_contains_static_update(property)
                        || Self::expression_contains_static_update(value)
                })
            || object_binding
                .property_descriptors
                .iter()
                .any(|(property, descriptor)| {
                    Self::expression_contains_static_update(property)
                        || descriptor
                            .value
                            .as_ref()
                            .is_some_and(Self::expression_contains_static_update)
                        || descriptor
                            .getter
                            .as_ref()
                            .is_some_and(Self::expression_contains_static_update)
                        || descriptor
                            .setter
                            .as_ref()
                            .is_some_and(Self::expression_contains_static_update)
                })
    }

    fn expression_targets_static_constructor_object(
        &self,
        expression: &Expression,
        target_name: &str,
        environment: &mut StaticResolutionEnvironment,
    ) -> bool {
        match expression {
            Expression::This => target_name == Self::STATIC_NEW_THIS_BINDING,
            Expression::Identifier(name) if name == target_name => true,
            Expression::Identifier(name) => {
                let Some(value) = environment.binding(name).cloned() else {
                    return false;
                };
                !static_expression_matches(&value, expression)
                    && self.expression_targets_static_constructor_object(
                        &value,
                        target_name,
                        environment,
                    )
            }
            Expression::Sequence(expressions) => expressions.last().is_some_and(|expression| {
                self.expression_targets_static_constructor_object(
                    expression,
                    target_name,
                    environment,
                )
            }),
            _ => false,
        }
    }

    fn static_constructor_member_key(
        &self,
        property: &Expression,
        environment: &mut StaticResolutionEnvironment,
    ) -> Option<Expression> {
        self.resolve_static_constructor_property_key(property, environment)
            .or_else(|| self.evaluate_static_expression_with_state(property, environment))
            .or_else(|| self.materialize_static_expression_with_state(property, environment))
            .and_then(|property| self.resolve_primitive_property_key_expression(&property))
            .or_else(|| self.resolve_primitive_property_key_expression(property))
    }

    fn static_constructor_known_member_property(
        &self,
        target_name: &str,
        property: &Expression,
        current_function_name: Option<&str>,
        environment: &StaticResolutionEnvironment,
    ) -> bool {
        if environment
            .object_binding(target_name)
            .is_some_and(|binding| {
                object_binding_lookup_value(binding, property).is_some()
                    || object_binding_lookup_descriptor(binding, property).is_some()
            })
        {
            return true;
        }

        let Some(current_function_name) = current_function_name else {
            return false;
        };
        self.resolve_function_prototype_object_binding(current_function_name)
            .as_ref()
            .is_some_and(|binding| {
                object_binding_lookup_value(binding, property).is_some()
                    || object_binding_lookup_descriptor(binding, property).is_some()
            })
    }

    fn static_constructor_member_call_missing_property(
        &self,
        expression: &Expression,
        target_name: &str,
        current_function_name: Option<&str>,
        environment: &mut StaticResolutionEnvironment,
    ) -> bool {
        match expression {
            Expression::Call { callee, .. } => {
                let Expression::Member { object, property } = callee.as_ref() else {
                    return false;
                };
                if !self.expression_targets_static_constructor_object(
                    object,
                    target_name,
                    environment,
                ) {
                    return false;
                }
                let Some(property) = self.static_constructor_member_key(property, environment)
                else {
                    return false;
                };
                !self.static_constructor_known_member_property(
                    target_name,
                    &property,
                    current_function_name,
                    environment,
                )
            }
            Expression::Sequence(expressions) => expressions.last().is_some_and(|expression| {
                self.static_constructor_member_call_missing_property(
                    expression,
                    target_name,
                    current_function_name,
                    environment,
                )
            }),
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => match self.evaluate_static_expression_with_state(condition, environment) {
                Some(Expression::Bool(true)) => self
                    .static_constructor_member_call_missing_property(
                        then_expression,
                        target_name,
                        current_function_name,
                        environment,
                    ),
                Some(Expression::Bool(false)) => self
                    .static_constructor_member_call_missing_property(
                        else_expression,
                        target_name,
                        current_function_name,
                        environment,
                    ),
                _ => false,
            },
            _ => false,
        }
    }

    fn resolve_static_constructor_property_key(
        &self,
        expression: &Expression,
        environment: &mut StaticResolutionEnvironment,
    ) -> Option<Expression> {
        let mut candidates = Vec::new();
        let mut push_candidate = |candidate: Expression| {
            if candidates
                .iter()
                .any(|existing| static_expression_matches(existing, &candidate))
            {
                return;
            }
            candidates.push(candidate);
        };

        push_candidate(expression.clone());

        if let Some(resolved) = self
            .resolve_bound_alias_expression_with_state(expression, environment)
            .filter(|resolved| !static_expression_matches(resolved, expression))
        {
            push_candidate(resolved.clone());
            if let Some(evaluated) =
                self.evaluate_static_expression_with_state(&resolved, environment)
            {
                push_candidate(evaluated);
            }
        }

        if let Some(evaluated) = self.evaluate_static_expression_with_state(expression, environment)
        {
            push_candidate(evaluated);
        }

        if let Some(materialized) =
            self.materialize_static_expression_with_state(expression, environment)
        {
            push_candidate(materialized.clone());
            if let Some(evaluated) =
                self.evaluate_static_expression_with_state(&materialized, environment)
            {
                push_candidate(evaluated);
            }
        }

        for candidate in &candidates {
            if let Some(key) = self.resolve_primitive_property_key_expression(candidate) {
                return Some(key);
            }
        }

        for candidate in &candidates {
            if let Some(object_binding) = self
                .resolve_object_binding_from_expression_with_state(candidate, environment)
                .or_else(|| {
                    let materialized =
                        self.materialize_static_expression_with_state(candidate, environment)?;
                    self.resolve_object_binding_from_expression_with_state(
                        &materialized,
                        environment,
                    )
                })
                && let Some((_, key)) =
                    self.resolve_property_key_coercion_from_object_binding(&object_binding)
            {
                return Some(key);
            }
        }

        None
    }

    fn apply_static_constructor_define_property_statement_updates(
        &self,
        statements: &[Statement],
        environment: &mut StaticResolutionEnvironment,
        current_function_name: Option<&str>,
        direct_eval_in_class_field_initializer: bool,
        strict_mode: bool,
    ) -> Result<(), StaticThrowValue> {
        for statement in statements {
            let terminal_statement =
                matches!(statement, Statement::Return(_) | Statement::Throw(_));
            match statement {
                Statement::Var { name, value } | Statement::Let { name, value, .. } => {
                    let value = self.resolve_static_constructor_binding_value(value, environment);
                    environment.set_local_binding(name.clone(), value);
                }
                Statement::Assign { name, value } => {
                    let value = self.resolve_static_constructor_binding_value(value, environment);
                    environment.assign_binding_value(name.clone(), value);
                }
                Statement::AssignMember {
                    object,
                    property,
                    value,
                } => {
                    let target_name = match object {
                        Expression::Identifier(target_name) => target_name.clone(),
                        Expression::This => Self::STATIC_NEW_THIS_BINDING.to_string(),
                        _ => continue,
                    };
                    let property = self
                        .resolve_static_constructor_property_key(property, environment)
                        .or_else(|| {
                            self.evaluate_static_expression_with_state(property, environment)
                        })
                        .or_else(|| {
                            self.materialize_static_expression_with_state(property, environment)
                        })
                        .unwrap_or_else(|| property.clone());
                    if self.static_constructor_member_call_missing_property(
                        value,
                        &target_name,
                        current_function_name,
                        environment,
                    ) {
                        return Err(StaticThrowValue::NamedError("TypeError"));
                    }
                    let this_binding =
                        Expression::Identifier(Self::STATIC_NEW_THIS_BINDING.to_string());
                    let value = self
                        .resolve_static_define_property_value_expression_with_eval_environment(
                            value,
                            current_function_name,
                            direct_eval_in_class_field_initializer,
                            strict_mode,
                            environment,
                            Some(&this_binding),
                        )
                        .or_else(|| self.evaluate_static_expression_with_state(value, environment))
                        .or_else(|| {
                            self.materialize_static_expression_with_state(value, environment)
                        })
                        .unwrap_or_else(|| {
                            self.resolve_static_constructor_binding_value(value, environment)
                        });
                    if let Some(object_binding) = environment.object_binding_mut(&target_name) {
                        if !object_binding_can_define_property(object_binding, &property) {
                            return Err(StaticThrowValue::NamedError("TypeError"));
                        }
                        object_binding_set_property(object_binding, property, value);
                    }
                }
                Statement::Declaration { body }
                | Statement::Block { body }
                | Statement::Labeled { body, .. }
                | Statement::With { body, .. } => {
                    self.apply_static_constructor_define_property_statement_updates(
                        body,
                        environment,
                        current_function_name,
                        direct_eval_in_class_field_initializer,
                        strict_mode,
                    )?;
                }
                Statement::If {
                    then_branch,
                    else_branch,
                    ..
                } => {
                    self.apply_static_constructor_define_property_statement_updates(
                        then_branch,
                        environment,
                        current_function_name,
                        direct_eval_in_class_field_initializer,
                        strict_mode,
                    )?;
                    self.apply_static_constructor_define_property_statement_updates(
                        else_branch,
                        environment,
                        current_function_name,
                        direct_eval_in_class_field_initializer,
                        strict_mode,
                    )?;
                }
                Statement::While { body, .. }
                | Statement::DoWhile { body, .. }
                | Statement::For { body, .. }
                | Statement::Try { body, .. } => {
                    self.apply_static_constructor_define_property_statement_updates(
                        body,
                        environment,
                        current_function_name,
                        direct_eval_in_class_field_initializer,
                        strict_mode,
                    )?;
                }
                Statement::Switch { cases, .. } => {
                    for case in cases {
                        self.apply_static_constructor_define_property_statement_updates(
                            &case.body,
                            environment,
                            current_function_name,
                            direct_eval_in_class_field_initializer,
                            strict_mode,
                        )?;
                    }
                }
                Statement::Expression(Expression::Call { callee, arguments })
                | Statement::Return(Expression::Call { callee, arguments })
                    if matches!(
                        callee.as_ref(),
                        Expression::Member { object, property }
                            if matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
                                && matches!(property.as_ref(), Expression::String(name) if name == "defineProperty")
                    ) =>
                {
                    let [
                        CallArgument::Expression(target_expression),
                        CallArgument::Expression(property_expression),
                        CallArgument::Expression(descriptor_expression),
                        ..,
                    ] = arguments.as_slice()
                    else {
                        continue;
                    };
                    let target_name = match target_expression {
                        Expression::Identifier(target_name) => target_name.clone(),
                        Expression::This => Self::STATIC_NEW_THIS_BINDING.to_string(),
                        _ => continue,
                    };
                    let Some(descriptor) =
                        resolve_property_descriptor_definition(descriptor_expression)
                    else {
                        continue;
                    };
                    let property = self
                        .resolve_static_constructor_property_key(property_expression, environment)
                        .or_else(|| {
                            self.evaluate_static_expression_with_state(
                                property_expression,
                                environment,
                            )
                        })
                        .or_else(|| {
                            self.materialize_static_expression_with_state(
                                property_expression,
                                environment,
                            )
                        })
                        .unwrap_or_else(|| property_expression.clone());
                    let enumerable = descriptor.enumerable.unwrap_or(false);
                    let this_binding =
                        Expression::Identifier(Self::STATIC_NEW_THIS_BINDING.to_string());
                    let value = if descriptor.is_accessor() {
                        Expression::Undefined
                    } else {
                        descriptor
                            .value
                            .as_ref()
                            .and_then(|expression| {
                                self
                                    .resolve_static_define_property_value_expression_with_eval_environment(
                                        expression,
                                        current_function_name,
                                        direct_eval_in_class_field_initializer,
                                        strict_mode,
                                        environment,
                                        Some(&this_binding),
                                    )
                                    .or_else(|| {
                                        self.evaluate_static_expression_with_state(
                                            expression,
                                            environment,
                                        )
                                    })
                                    .or_else(|| {
                                        self.materialize_static_expression_with_state(
                                            expression,
                                            environment,
                                        )
                                    })
                            })
                            .unwrap_or(Expression::Undefined)
                    };
                    if let Some(object_binding) = environment.object_binding_mut(&target_name) {
                        if !object_binding_can_define_property(object_binding, &property) {
                            return Err(StaticThrowValue::NamedError("TypeError"));
                        }
                        object_binding_define_property(object_binding, property, value, enumerable);
                    }
                }
                _ => {}
            }
            if terminal_statement {
                break;
            }
        }
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn resolve_user_constructor_return_expression_with_explicit_status_for_function(
        &self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        capture_source_bindings: Option<&HashMap<String, Expression>>,
    ) -> Option<(Expression, bool)> {
        let trace_constructor_return = std::env::var_os("AYY_TRACE_CONSTRUCTOR_RETURN").is_some();
        if !user_function.is_constructible() {
            if trace_constructor_return {
                eprintln!(
                    "constructor_return:{}:not_constructible",
                    user_function.name
                );
            }
            return None;
        }

        let this_name = Self::STATIC_NEW_THIS_BINDING.to_string();
        let this_binding = Expression::Identifier(this_name.clone());
        let mut extra_local_bindings = HashMap::new();
        extra_local_bindings.insert(
            Self::STATIC_NEW_THIS_INITIALIZED_BINDING.to_string(),
            Expression::Bool(false),
        );
        let mut execution = self.prepare_static_user_function_execution(
            &user_function.name,
            user_function,
            arguments,
            &this_binding,
            capture_source_bindings,
            extra_local_bindings,
            |statement| Self::substitute_static_constructor_new_target_statement(&statement),
        )?;
        if trace_constructor_return {
            eprintln!(
                "constructor_return:{}:body={:?}",
                user_function.name, execution.substituted_body
            );
        }

        let assigned_nonlocal_bindings =
            self.collect_user_function_assigned_nonlocal_bindings(user_function);
        let direct_eval_in_class_field_initializer = self
            .resolve_registered_function_declaration(&user_function.name)
            .is_some_and(|declaration| declaration.direct_eval_in_class_field_initializer);
        let has_snapshot_call = execution
            .substituted_body
            .iter()
            .any(Self::statement_contains_static_constructor_snapshot_call);
        let blocking_assigned_nonlocal_bindings =
            Self::static_constructor_snapshot_blocking_assigned_nonlocal_bindings(
                &execution.substituted_body,
                &assigned_nonlocal_bindings,
            );
        if !blocking_assigned_nonlocal_bindings.is_empty()
            && has_snapshot_call
            && !direct_eval_in_class_field_initializer
        {
            if trace_constructor_return {
                eprintln!(
                    "constructor_return:{}:blocked_nonlocals={:?}",
                    user_function.name, blocking_assigned_nonlocal_bindings
                );
            }
            return None;
        }

        execution
            .environment
            .set_local_object_binding(this_name.clone(), empty_object_value_binding());

        let return_value = self.execute_static_statements_with_state(
            &execution.substituted_body,
            &mut execution.environment,
        )?;
        if trace_constructor_return {
            eprintln!(
                "constructor_return:{}:return_value={:?} locals={:?} local_objects={:?} global_objects={:?}",
                user_function.name,
                return_value,
                execution.environment.local_bindings,
                execution
                    .environment
                    .local_object_bindings
                    .keys()
                    .collect::<Vec<_>>(),
                execution
                    .environment
                    .global_object_bindings
                    .keys()
                    .collect::<Vec<_>>()
            );
        }
        if self
            .apply_static_constructor_define_property_statement_updates(
                &execution.substituted_body,
                &mut execution.environment,
                Some(&user_function.name),
                direct_eval_in_class_field_initializer,
                user_function.strict,
            )
            .is_err()
        {
            if trace_constructor_return {
                eprintln!(
                    "constructor_return:{}:define_property_failed",
                    user_function.name
                );
            }
            return None;
        }
        if return_value.is_none()
            && self.user_function_is_derived_constructor(user_function)
            && matches!(
                execution
                    .environment
                    .local_binding(&Self::STATIC_NEW_THIS_INITIALIZED_BINDING.to_string()),
                Some(Expression::Bool(true))
            )
            && let Some(this_expression) = execution.environment.local_binding(&this_name).cloned()
            && !matches!(
                &this_expression,
                Expression::Identifier(name) if name == &this_name
            )
            && self
                .resolve_object_binding_from_expression_with_state(
                    &this_expression,
                    &mut execution.environment,
                )
                .is_some()
        {
            return Some((this_expression, false));
        }
        let Some(return_value) = return_value else {
            if trace_constructor_return {
                eprintln!("constructor_return:{}:no_return_value", user_function.name);
            }
            return None;
        };
        if self
            .resolve_object_binding_from_expression_with_state(
                &return_value,
                &mut execution.environment,
            )
            .is_some()
            && let Some(alias) =
                Self::static_constructor_return_assignment_alias(&execution.substituted_body)
            && let Some(alias_value) = execution.environment.binding(&alias).cloned()
            && self.static_constructor_return_value_matches_alias(
                &alias_value,
                &return_value,
                &mut execution.environment,
            )
        {
            return Some((Expression::Identifier(alias), true));
        }
        Some((return_value, true))
    }

    pub(in crate::backend::direct_wasm) fn resolve_user_constructor_return_expression_for_function(
        &self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        capture_source_bindings: Option<&HashMap<String, Expression>>,
    ) -> Option<Expression> {
        self.resolve_user_constructor_return_expression_with_explicit_status_for_function(
            user_function,
            arguments,
            capture_source_bindings,
        )
        .map(|(expression, _)| expression)
    }

    pub(in crate::backend::direct_wasm) fn resolve_user_constructor_explicit_return_expression_for_function(
        &self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        capture_source_bindings: Option<&HashMap<String, Expression>>,
    ) -> Option<Expression> {
        self.resolve_user_constructor_return_expression_with_explicit_status_for_function(
            user_function,
            arguments,
            capture_source_bindings,
        )
        .and_then(|(expression, explicit)| explicit.then_some(expression))
    }

    pub(in crate::backend::direct_wasm) fn resolve_user_constructor_object_binding_for_function(
        &self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        capture_source_bindings: Option<&HashMap<String, Expression>>,
    ) -> Option<ObjectValueBinding> {
        self.resolve_user_constructor_object_binding_outcome_for_function(
            user_function,
            arguments,
            capture_source_bindings,
        )
        .and_then(Result::ok)
    }

    pub(in crate::backend::direct_wasm) fn resolve_user_constructor_object_binding_outcome_for_function(
        &self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        capture_source_bindings: Option<&HashMap<String, Expression>>,
    ) -> Option<Result<ObjectValueBinding, StaticThrowValue>> {
        self.resolve_user_constructor_object_binding_outcome_for_function_with_this_binding(
            user_function,
            arguments,
            capture_source_bindings,
            empty_object_value_binding(),
        )
    }

    pub(in crate::backend::direct_wasm) fn resolve_user_constructor_object_binding_for_function_with_this_binding(
        &self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        capture_source_bindings: Option<&HashMap<String, Expression>>,
        this_object_binding: ObjectValueBinding,
    ) -> Option<ObjectValueBinding> {
        self.resolve_user_constructor_object_binding_outcome_for_function_with_this_binding(
            user_function,
            arguments,
            capture_source_bindings,
            this_object_binding,
        )
        .and_then(Result::ok)
    }

    pub(in crate::backend::direct_wasm) fn resolve_user_constructor_object_binding_outcome_for_function_with_this_binding(
        &self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        capture_source_bindings: Option<&HashMap<String, Expression>>,
        this_object_binding: ObjectValueBinding,
    ) -> Option<Result<ObjectValueBinding, StaticThrowValue>> {
        let trace_private = std::env::var_os("AYY_TRACE_PRIVATE_MEMBER_LOOKUP").is_some();
        if !user_function.is_constructible() {
            return None;
        }

        let this_name = Self::STATIC_NEW_THIS_BINDING.to_string();
        let this_binding = Expression::Identifier(this_name.clone());
        let mut extra_local_bindings = HashMap::new();
        extra_local_bindings.insert(
            Self::STATIC_NEW_THIS_INITIALIZED_BINDING.to_string(),
            Expression::Bool(false),
        );
        let mut execution = self.prepare_static_user_function_execution(
            &user_function.name,
            user_function,
            arguments,
            &this_binding,
            capture_source_bindings,
            extra_local_bindings,
            |statement| Self::substitute_static_constructor_new_target_statement(&statement),
        )?;

        let direct_eval_in_class_field_initializer = self
            .resolve_registered_function_declaration(&user_function.name)
            .is_some_and(|declaration| declaration.direct_eval_in_class_field_initializer);
        let assigned_nonlocal_bindings =
            self.collect_user_function_assigned_nonlocal_bindings(user_function);
        let has_snapshot_call = execution
            .substituted_body
            .iter()
            .any(Self::statement_contains_static_constructor_snapshot_call);
        let blocking_assigned_nonlocal_bindings =
            Self::static_constructor_snapshot_blocking_assigned_nonlocal_bindings(
                &execution.substituted_body,
                &assigned_nonlocal_bindings,
            );
        if !blocking_assigned_nonlocal_bindings.is_empty()
            && has_snapshot_call
            && !direct_eval_in_class_field_initializer
        {
            return None;
        }

        let mut seeded_this_object_binding = this_object_binding.clone();
        if !self.user_function_is_derived_constructor(user_function)
            && this_object_binding.extensible
        {
            self.seed_constructed_private_member_markers(
                &user_function.name,
                &mut seeded_this_object_binding,
            );
        }
        if trace_private {
            eprintln!(
                "private_constructor_object:start function={} derived={} seeded_props={:?}",
                user_function.name,
                self.user_function_is_derived_constructor(user_function),
                ordered_object_property_names(&seeded_this_object_binding),
            );
        }
        execution
            .environment
            .set_local_object_binding(this_name.clone(), seeded_this_object_binding);

        let mut preflight_environment = execution.environment.clone();
        if let Err(throw_value) = self.apply_static_constructor_define_property_statement_updates(
            &execution.substituted_body,
            &mut preflight_environment,
            Some(&user_function.name),
            direct_eval_in_class_field_initializer,
            user_function.strict,
        ) {
            return Some(Err(throw_value));
        }

        let return_value = self.execute_static_statements_with_state(
            &execution.substituted_body,
            &mut execution.environment,
        );
        if let Err(throw_value) = self.apply_static_constructor_define_property_statement_updates(
            &execution.substituted_body,
            &mut execution.environment,
            Some(&user_function.name),
            direct_eval_in_class_field_initializer,
            user_function.strict,
        ) {
            return Some(Err(throw_value));
        }
        if trace_private {
            eprintln!(
                "private_constructor_object:after function={} return={:?} props={:?}",
                user_function.name,
                return_value,
                execution
                    .environment
                    .object_binding(&this_name)
                    .map(ordered_object_property_names)
                    .unwrap_or_default(),
            );
        }
        if return_value.is_none() && direct_eval_in_class_field_initializer {
            return execution
                .environment
                .object_binding(&this_name)
                .cloned()
                .map(|binding| {
                    Self::static_constructor_object_binding_with_capture_sources(
                        binding,
                        capture_source_bindings,
                    )
                })
                .map(Ok);
        }
        if return_value.is_none()
            && self.user_function_is_derived_constructor(user_function)
            && execution
                .environment
                .object_binding(&this_name)
                .is_some_and(|binding| binding != &this_object_binding)
        {
            return execution
                .environment
                .object_binding(&this_name)
                .cloned()
                .map(|binding| {
                    Self::static_constructor_object_binding_with_capture_sources(
                        binding,
                        capture_source_bindings,
                    )
                })
                .map(Ok);
        }
        let return_value = return_value?;
        if let Some(return_value) = return_value
            && let Some(mut returned_object) = self
                .resolve_object_binding_from_expression_with_state(
                    &return_value,
                    &mut execution.environment,
                )
        {
            if let Some(this_object_binding) = execution.environment.object_binding(&this_name) {
                Self::expand_static_constructor_this_references_in_object_binding(
                    &mut returned_object,
                    &this_name,
                    this_object_binding,
                );
            }
            let returned_object = Self::static_constructor_object_binding_with_capture_sources(
                returned_object,
                capture_source_bindings,
            );
            return Some(Ok(returned_object));
        }
        execution
            .environment
            .object_binding(&this_name)
            .cloned()
            .map(|binding| {
                Self::static_constructor_object_binding_with_capture_sources(
                    binding,
                    capture_source_bindings,
                )
            })
            .map(Ok)
    }

    pub(in crate::backend::direct_wasm) fn resolve_user_constructor_updated_bindings_for_function(
        &self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        capture_source_bindings: Option<&HashMap<String, Expression>>,
    ) -> Option<HashMap<String, Expression>> {
        if !user_function.is_constructible() {
            return None;
        }

        let this_name = Self::STATIC_NEW_THIS_BINDING.to_string();
        let this_binding = Expression::Identifier(this_name.clone());
        let mut extra_local_bindings = HashMap::new();
        extra_local_bindings.insert(
            Self::STATIC_NEW_THIS_INITIALIZED_BINDING.to_string(),
            Expression::Bool(false),
        );
        let mut execution = self.prepare_static_user_function_execution(
            &user_function.name,
            user_function,
            arguments,
            &this_binding,
            capture_source_bindings,
            extra_local_bindings,
            |statement| Self::substitute_static_constructor_new_target_statement(&statement),
        )?;
        let direct_eval_in_class_field_initializer = self
            .resolve_registered_function_declaration(&user_function.name)
            .is_some_and(|declaration| declaration.direct_eval_in_class_field_initializer);
        let assigned_nonlocal_bindings =
            self.collect_user_function_assigned_nonlocal_bindings(user_function);
        let has_snapshot_call = execution
            .substituted_body
            .iter()
            .any(Self::statement_contains_static_constructor_snapshot_call);
        let blocking_assigned_nonlocal_bindings =
            Self::static_constructor_snapshot_blocking_assigned_nonlocal_bindings(
                &execution.substituted_body,
                &assigned_nonlocal_bindings,
            );
        if !blocking_assigned_nonlocal_bindings.is_empty()
            && has_snapshot_call
            && !direct_eval_in_class_field_initializer
        {
            return None;
        }

        execution
            .environment
            .set_local_object_binding(this_name.clone(), empty_object_value_binding());
        let _ = self.execute_static_statements_with_state(
            &execution.substituted_body,
            &mut execution.environment,
        );
        if self
            .apply_static_constructor_define_property_statement_updates(
                &execution.substituted_body,
                &mut execution.environment,
                Some(&user_function.name),
                direct_eval_in_class_field_initializer,
                user_function.strict,
            )
            .is_err()
        {
            return None;
        }
        let mut updated_bindings = HashMap::new();
        let updated_names = self.collect_user_function_updated_nonlocal_bindings(user_function);
        for name in updated_names {
            let source_name = scoped_binding_source_name(&name)
                .unwrap_or(&name)
                .to_string();
            if source_name == "this"
                || source_name == "arguments"
                || user_function.scope_bindings.contains(&source_name)
            {
                continue;
            }
            if let Some(mut object_binding) =
                execution.environment.object_binding(&source_name).cloned()
            {
                if let Some(this_object_binding) = execution.environment.object_binding(&this_name)
                {
                    Self::expand_static_constructor_this_references_in_object_binding(
                        &mut object_binding,
                        &this_name,
                        this_object_binding,
                    );
                }
                updated_bindings.insert(source_name, object_binding_to_expression(&object_binding));
            } else if let Some(value) = execution.environment.binding(&source_name).cloned() {
                updated_bindings.insert(source_name, value);
            }
        }

        Some(updated_bindings)
    }

    pub(in crate::backend::direct_wasm) fn resolve_user_constructor_object_binding_from_new(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Option<ObjectValueBinding> {
        let trace_constructor = std::env::var_os("AYY_TRACE_CONSTRUCTOR_BINDINGS").is_some();
        let LocalFunctionBinding::User(function_name) =
            self.resolve_function_binding_from_expression(callee)?
        else {
            return None;
        };
        let user_function = self.user_function(&function_name)?;
        if trace_constructor {
            eprintln!(
                "constructor_new_object:start callee={callee:?} function={function_name} derived={} args={arguments:?}",
                self.user_function_is_derived_constructor(user_function)
            );
        }
        let capture_source_bindings =
            self.resolve_constructor_capture_source_bindings_from_expression(callee);
        let snapshot_result = self
            .state
            .speculation
            .static_semantics
            .last_bound_user_function_call
            .as_ref()
            .filter(|snapshot| snapshot.function_name == function_name)
            .filter(|snapshot| {
                matches!(
                    snapshot.source_expression.as_ref(),
                    Some(Expression::New {
                        callee: snapshot_callee,
                        arguments: snapshot_arguments,
                    }) if static_expression_matches(snapshot_callee, callee)
                        && snapshot_arguments == arguments
                )
            })
            .and_then(|snapshot| snapshot.result_expression.as_ref())
            .and_then(|result| self.resolve_object_binding_from_expression(result));
        if trace_constructor {
            eprintln!(
                "constructor_new_object:snapshot function={function_name} props={:?}",
                snapshot_result
                    .as_ref()
                    .map(ordered_object_property_names)
                    .unwrap_or_default()
            );
        }
        if snapshot_result
            .as_ref()
            .is_some_and(|binding| !Self::object_binding_contains_static_update(binding))
            && !self.user_function_is_derived_constructor(user_function)
            && !Self::constructor_capture_sources_include_live_bindings(
                capture_source_bindings.as_ref(),
            )
        {
            if trace_constructor {
                eprintln!("constructor_new_object:return_snapshot function={function_name}");
            }
            return snapshot_result.map(|binding| {
                Self::static_constructor_object_binding_with_capture_sources(
                    binding,
                    capture_source_bindings.as_ref(),
                )
            });
        }
        let result = self.resolve_user_constructor_object_binding_for_function(
            user_function,
            arguments,
            capture_source_bindings.as_ref(),
        );
        if trace_constructor {
            eprintln!(
                "constructor_new_object:resolved function={function_name} result_props={:?}",
                result
                    .as_ref()
                    .map(ordered_object_property_names)
                    .unwrap_or_default()
            );
        }
        result.or_else(|| {
            snapshot_result.map(|binding| {
                Self::static_constructor_object_binding_with_capture_sources(
                    binding,
                    capture_source_bindings.as_ref(),
                )
            })
        })
    }

    pub(in crate::backend::direct_wasm) fn resolve_constructor_capture_source_bindings_from_expression(
        &self,
        expression: &Expression,
    ) -> Option<HashMap<String, Expression>> {
        let expression_capture_slots = self.resolve_function_expression_capture_slots(expression);
        let resolved = if expression_capture_slots.is_some() {
            expression.clone()
        } else {
            self.resolve_bound_alias_expression(expression)
                .or_else(|| match expression {
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
                .unwrap_or_else(|| expression.clone())
        };
        let mut call_arguments = None;
        let callee = match &resolved {
            Expression::Call { callee, arguments } | Expression::New { callee, arguments } => {
                call_arguments = Some(arguments.as_slice());
                callee.as_ref()
            }
            _ => &resolved,
        };
        let LocalFunctionBinding::User(function_name) =
            self.resolve_function_binding_from_expression(callee)?
        else {
            return None;
        };
        let user_function = self.user_function(&function_name)?;
        if call_arguments.is_none() {
            let constructor_slot_owner = match expression {
                Expression::Identifier(name) => Some(name),
                _ => match callee {
                    Expression::Identifier(name) => Some(name),
                    _ => None,
                },
            };
            let constructor_capture_slots = if let Some(name) = constructor_slot_owner {
                let constructor_property = Expression::String("constructor".to_string());
                let prototype = Expression::Member {
                    object: Box::new(Expression::Identifier(name.clone())),
                    property: Box::new(Expression::String("prototype".to_string())),
                };
                self.resolve_member_function_capture_slots(&prototype, &constructor_property)
            } else {
                None
            };
            let capture_source_names = self
                .user_function_capture_bindings(&function_name)
                .filter(|captures| !captures.is_empty())
                .map(|captures| captures.keys().cloned().collect::<Vec<_>>())
                .or_else(|| {
                    let function = self.resolve_registered_function_declaration(&function_name)?;
                    let mut names =
                        collect_referenced_binding_names_from_statements(&function.body)
                            .into_iter()
                            .map(|name| {
                                scoped_binding_source_name(&name)
                                    .unwrap_or(&name)
                                    .to_string()
                            })
                            .filter(|name| {
                                name != "this"
                                    && name != "arguments"
                                    && !user_function.scope_bindings.contains(name)
                            })
                            .collect::<HashSet<_>>();
                    names.retain(|name| self.user_function_capture_source_is_locally_bound(name));
                    (!names.is_empty()).then_some(names.into_iter().collect::<Vec<_>>())
                })?;
            let mut bindings = HashMap::new();
            for source_name in capture_source_names {
                let direct_class_brand_slot = expression_capture_slots
                    .as_ref()
                    .and_then(|capture_slots| capture_slots.get(&source_name))
                    .filter(|slot_name| {
                        source_name.starts_with("__ayy_class_brand_")
                            && slot_name.as_str() == source_name.as_str()
                    });
                if let Some(slot_name) = direct_class_brand_slot
                    .or_else(|| {
                        expression_capture_slots
                            .as_ref()
                            .and_then(|capture_slots| capture_slots.get(&source_name))
                            .filter(|slot_name| slot_name.as_str() != source_name.as_str())
                    })
                    .or_else(|| {
                        constructor_capture_slots
                            .as_ref()
                            .and_then(|capture_slots| capture_slots.get(&source_name))
                    })
                    .or_else(|| {
                        expression_capture_slots
                            .as_ref()
                            .and_then(|capture_slots| capture_slots.get(&source_name))
                    })
                {
                    let source_expression = if slot_name == &source_name
                        && source_name.starts_with("__ayy_class_brand_")
                    {
                        Expression::Identifier(source_name.clone())
                    } else if slot_name == &source_name
                        && !Self::constructor_capture_source_is_stable_snapshot(&source_name)
                    {
                        self.current_constructor_capture_source_expression(
                            &source_name,
                            constructor_slot_owner,
                        )
                    } else {
                        let snapshot = self.snapshot_bound_capture_slot_expression(slot_name);
                        if matches!(&snapshot, Expression::Identifier(name) if name == &source_name)
                        {
                            self.resolve_bound_alias_expression(&snapshot)
                                .filter(|resolved| !static_expression_matches(resolved, &snapshot))
                                .or_else(|| {
                                    source_name.starts_with("__ayy_class_super_").then(|| {
                                        constructor_slot_owner.and_then(|owner| {
                                        self.resolve_constructor_synthetic_super_source_expression(
                                            owner,
                                        )
                                    })
                                    })?
                                })
                                .or_else(|| self.global_value_binding(&source_name).cloned())
                                .unwrap_or(snapshot)
                        } else if matches!(&snapshot, Expression::Identifier(name) if name == slot_name)
                            && source_name.starts_with("__ayy_class_super_")
                        {
                            constructor_slot_owner
                                .and_then(|owner| {
                                    self.resolve_constructor_synthetic_super_source_expression(
                                        owner,
                                    )
                                })
                                .unwrap_or(snapshot)
                        } else {
                            snapshot
                        }
                    };
                    if std::env::var_os("AYY_TRACE_CAPTURE_BINDINGS").is_some() {
                        eprintln!(
                            "constructor_capture_source expression={expression:?} source={source_name} slot={slot_name} resolved={source_expression:?}"
                        );
                    }
                    bindings.insert(source_name, source_expression);
                    continue;
                }
                let source_expression = if source_name == "this" {
                    Expression::This
                } else if source_name == "new.target" {
                    Expression::NewTarget
                } else {
                    Expression::Identifier(source_name.clone())
                };
                let resolved_source = self
                    .resolve_bound_alias_expression(&source_expression)
                    .filter(|resolved| !static_expression_matches(resolved, &source_expression))
                    .unwrap_or(source_expression);
                bindings.insert(source_name, resolved_source);
            }
            return Some(bindings);
        }
        let arguments = call_arguments.expect("filtered above");
        let mut execution = self.prepare_static_user_function_execution(
            &function_name,
            user_function,
            arguments,
            &Expression::Undefined,
            None,
            HashMap::new(),
            |statement| statement,
        )?;
        self.execute_static_statements_with_state(
            &execution.substituted_body,
            &mut execution.environment,
        )?;
        Some(execution.environment.into_local_bindings())
    }
}
