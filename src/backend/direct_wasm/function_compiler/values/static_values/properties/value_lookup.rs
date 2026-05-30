use super::*;

impl<'a> FunctionCompiler<'a> {
    fn static_statement_binding_source_expression_from_statement(
        statement: &Statement,
        binding_name: &str,
    ) -> Option<Expression> {
        match statement {
            Statement::Let { name, value, .. }
            | Statement::Var { name, value }
            | Statement::Assign { name, value }
                if name == binding_name =>
            {
                Some(value.clone())
            }
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => {
                Self::static_statement_binding_source_expression_from_statements(body, binding_name)
            }
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => Self::static_statement_binding_source_expression_from_statements(
                then_branch,
                binding_name,
            )
            .or_else(|| {
                Self::static_statement_binding_source_expression_from_statements(
                    else_branch,
                    binding_name,
                )
            }),
            Statement::With { body, .. }
            | Statement::While { body, .. }
            | Statement::DoWhile { body, .. } => {
                Self::static_statement_binding_source_expression_from_statements(body, binding_name)
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                Self::static_statement_binding_source_expression_from_statements(body, binding_name)
                    .or_else(|| {
                        Self::static_statement_binding_source_expression_from_statements(
                            catch_setup,
                            binding_name,
                        )
                    })
                    .or_else(|| {
                        Self::static_statement_binding_source_expression_from_statements(
                            catch_body,
                            binding_name,
                        )
                    })
            }
            Statement::Switch { cases, .. } => cases.iter().find_map(|case| {
                Self::static_statement_binding_source_expression_from_statements(
                    &case.body,
                    binding_name,
                )
            }),
            Statement::For { init, body, .. } => {
                Self::static_statement_binding_source_expression_from_statements(init, binding_name)
                    .or_else(|| {
                        Self::static_statement_binding_source_expression_from_statements(
                            body,
                            binding_name,
                        )
                    })
            }
            Statement::Assign { .. }
            | Statement::AssignMember { .. }
            | Statement::Let { .. }
            | Statement::Var { .. }
            | Statement::Print { .. }
            | Statement::Expression(_)
            | Statement::Throw(_)
            | Statement::Return(_)
            | Statement::Break { .. }
            | Statement::Continue { .. }
            | Statement::Yield { .. }
            | Statement::YieldDelegate { .. } => None,
        }
    }

    fn static_statement_binding_source_expression_from_statements(
        statements: &[Statement],
        binding_name: &str,
    ) -> Option<Expression> {
        statements.iter().find_map(|statement| {
            Self::static_statement_binding_source_expression_from_statement(statement, binding_name)
        })
    }

    fn resolve_generated_class_field_source_expression_from_statements(
        expression: Expression,
        statements: &[Statement],
        depth: usize,
    ) -> Expression {
        if depth == 0 {
            return expression;
        }
        match expression {
            Expression::Identifier(name) => {
                let Some(source) = Self::static_statement_binding_source_expression_from_statements(
                    statements, &name,
                ) else {
                    return Expression::Identifier(name);
                };
                if matches!(&source, Expression::Identifier(source_name) if source_name == &name) {
                    return source;
                }
                Self::resolve_generated_class_field_source_expression_from_statements(
                    source,
                    statements,
                    depth - 1,
                )
            }
            Expression::Sequence(expressions) => Expression::Sequence(
                expressions
                    .into_iter()
                    .map(|expression| {
                        Self::resolve_generated_class_field_source_expression_from_statements(
                            expression, statements, depth,
                        )
                    })
                    .collect(),
            ),
            Expression::Assign { name, value } => Expression::Assign {
                name,
                value: Box::new(
                    Self::resolve_generated_class_field_source_expression_from_statements(
                        *value, statements, depth,
                    ),
                ),
            },
            Expression::Binary { op, left, right } => Expression::Binary {
                op,
                left: Box::new(
                    Self::resolve_generated_class_field_source_expression_from_statements(
                        *left, statements, depth,
                    ),
                ),
                right: Box::new(
                    Self::resolve_generated_class_field_source_expression_from_statements(
                        *right, statements, depth,
                    ),
                ),
            },
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => Expression::Conditional {
                condition: Box::new(
                    Self::resolve_generated_class_field_source_expression_from_statements(
                        *condition, statements, depth,
                    ),
                ),
                then_expression: Box::new(
                    Self::resolve_generated_class_field_source_expression_from_statements(
                        *then_expression,
                        statements,
                        depth,
                    ),
                ),
                else_expression: Box::new(
                    Self::resolve_generated_class_field_source_expression_from_statements(
                        *else_expression,
                        statements,
                        depth,
                    ),
                ),
            },
            _ => expression,
        }
    }

    fn generated_class_field_source_expression_from_statement(
        statement: &Statement,
        capture_name: &str,
    ) -> Option<Expression> {
        match statement {
            Statement::Let { name, value, .. } | Statement::Var { name, value }
                if name == capture_name =>
            {
                Some(value.clone())
            }
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => {
                Self::generated_class_field_source_expression_from_statements(body, capture_name)
            }
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => Self::generated_class_field_source_expression_from_statements(
                then_branch,
                capture_name,
            )
            .or_else(|| {
                Self::generated_class_field_source_expression_from_statements(
                    else_branch,
                    capture_name,
                )
            }),
            Statement::With { body, .. }
            | Statement::While { body, .. }
            | Statement::DoWhile { body, .. } => {
                Self::generated_class_field_source_expression_from_statements(body, capture_name)
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => Self::generated_class_field_source_expression_from_statements(body, capture_name)
                .or_else(|| {
                    Self::generated_class_field_source_expression_from_statements(
                        catch_setup,
                        capture_name,
                    )
                })
                .or_else(|| {
                    Self::generated_class_field_source_expression_from_statements(
                        catch_body,
                        capture_name,
                    )
                }),
            Statement::Switch { cases, .. } => cases.iter().find_map(|case| {
                Self::generated_class_field_source_expression_from_statements(
                    &case.body,
                    capture_name,
                )
            }),
            Statement::For { init, body, .. } => {
                Self::generated_class_field_source_expression_from_statements(init, capture_name)
                    .or_else(|| {
                        Self::generated_class_field_source_expression_from_statements(
                            body,
                            capture_name,
                        )
                    })
            }
            Statement::Assign { .. }
            | Statement::AssignMember { .. }
            | Statement::Let { .. }
            | Statement::Var { .. }
            | Statement::Print { .. }
            | Statement::Expression(_)
            | Statement::Throw(_)
            | Statement::Return(_)
            | Statement::Break { .. }
            | Statement::Continue { .. }
            | Statement::Yield { .. }
            | Statement::YieldDelegate { .. } => None,
        }
    }

    fn generated_class_field_source_expression_from_statements(
        statements: &[Statement],
        capture_name: &str,
    ) -> Option<Expression> {
        statements.iter().find_map(|statement| {
            Self::generated_class_field_source_expression_from_statement(statement, capture_name)
        })
    }

    fn global_alias_for_identifier_value(&self, target_name: &str) -> Option<String> {
        self.backend
            .global_semantics
            .values
            .value_bindings
            .iter()
            .find_map(|(name, value)| {
                matches!(value, Expression::Identifier(value_name) if value_name == target_name)
                    .then(|| name.clone())
            })
    }

    fn global_member_capture_source_expression(&self, capture_name: &str) -> Option<Expression> {
        self.backend
            .global_member_function_capture_slot_entries()
            .into_iter()
            .find_map(|(key, capture_slots)| {
                let slot_name = capture_slots.get(capture_name)?;
                let target_name = match &key.target {
                    MemberFunctionBindingTarget::Identifier(name)
                    | MemberFunctionBindingTarget::Prototype(name) => name,
                };
                let source_from_target = self
                    .resolve_constructor_capture_source_bindings_from_expression(
                        &Expression::Identifier(target_name.clone()),
                    )
                    .and_then(|bindings| bindings.get(capture_name).cloned())
                    .filter(|source| {
                        !matches!(source, Expression::Identifier(name) if name == capture_name)
                    });
                source_from_target.or_else(|| {
                    let slot_name = slot_name.clone();
                    let slot_identifier = Expression::Identifier(slot_name.clone());
                    self.resolve_capture_slot_static_source_expression(&slot_name)
                        .or_else(|| {
                            self.resolve_bound_alias_expression(&slot_identifier)
                                .filter(|source| {
                                    !static_expression_matches(source, &slot_identifier)
                                })
                                .filter(|source| {
                                    matches!(source, Expression::Identifier(name) if self.lookup_identifier_kind(name) == Some(StaticValueKind::Symbol))
                                })
                        })
                        .or_else(|| {
                            let snapshot = self.snapshot_bound_capture_slot_expression(&slot_name);
                            (!matches!(
                                &snapshot,
                                Expression::Identifier(name)
                                    if name == capture_name || name == &slot_name
                            ))
                            .then_some(snapshot)
                        })
                })
            })
    }

    pub(in crate::backend::direct_wasm) fn generated_class_field_source_expression(
        &self,
        capture_name: &str,
    ) -> Option<Expression> {
        if !capture_name.starts_with("__ayy_class_field_name_") {
            return None;
        }
        self.backend
            .function_registry
            .catalog
            .registered_function_declarations
            .iter()
            .find_map(|function| {
                let source = Self::generated_class_field_source_expression_from_statements(
                    &function.body,
                    capture_name,
                )?;
                Some(
                    Self::resolve_generated_class_field_source_expression_from_statements(
                        source,
                        &function.body,
                        8,
                    ),
                )
            })
    }

    fn resolve_object_binding_constructor_capture_symbol_value(
        &self,
        object: &Expression,
        object_binding: &ObjectValueBinding,
        property: &Expression,
    ) -> Option<Expression> {
        let canonical_property = self.canonical_object_property_expression(property);
        let requested_symbol = self
            .resolve_symbol_identity_expression(&canonical_property)
            .or_else(|| self.resolve_symbol_identity_expression(property))?;
        let capture_source_bindings = self
            .resolve_constructor_capture_source_bindings_from_expression(object)
            .or_else(|| {
                let Expression::Identifier(name) = object else {
                    return None;
                };
                let Expression::New { callee, .. } = self.global_value_binding(name)? else {
                    return None;
                };
                self.resolve_constructor_capture_source_bindings_from_expression(callee)
                    .or_else(|| {
                        let Expression::Identifier(constructor_name) = callee.as_ref() else {
                            return None;
                        };
                        let function =
                            self.resolve_registered_function_declaration(constructor_name)?;
                        function
                            .top_level_binding
                            .as_ref()
                            .into_iter()
                            .chain(function.self_binding.as_ref())
                            .filter_map(|binding_name| {
                                self.resolve_constructor_capture_source_bindings_from_expression(
                                    &Expression::Identifier(binding_name.clone()),
                                )
                            })
                            .next()
                            .or_else(|| {
                                let self_binding = function.self_binding.as_ref()?;
                                let alias = self.global_alias_for_identifier_value(self_binding)?;
                                self.resolve_constructor_capture_source_bindings_from_expression(
                                    &Expression::Identifier(alias),
                                )
                            })
                    })
            });
        let capture_source_bindings = capture_source_bindings?;
        object_binding
            .symbol_properties
            .iter()
            .find_map(|(existing_key, value)| {
                let Expression::Identifier(existing_name) = existing_key else {
                    return None;
                };
                let source_expression = capture_source_bindings
                    .get(existing_name)
                    .filter(|source| {
                        !matches!(source, Expression::Identifier(name) if name == existing_name)
                    })
                    .cloned()
                    .or_else(|| self.generated_class_field_source_expression(existing_name))
                    .or_else(|| self.global_member_capture_source_expression(existing_name))
                    .or_else(|| capture_source_bindings.get(existing_name).cloned())?;
                let source_symbol = self
                    .resolve_symbol_identity_expression(&source_expression)
                    .unwrap_or_else(|| source_expression.clone());
                (static_expression_matches(&source_symbol, &requested_symbol)
                    || static_expression_matches(&source_expression, &requested_symbol)
                    || static_expression_matches(&source_expression, property)
                    || static_expression_matches(&source_expression, &canonical_property))
                .then(|| value.clone())
            })
    }

    pub(in crate::backend::direct_wasm) fn resolve_object_binding_property_value(
        &self,
        object_binding: &ObjectValueBinding,
        property: &Expression,
    ) -> Option<Expression> {
        let canonical_property = self.canonical_object_property_expression(property);
        let requested_symbol = self
            .resolve_symbol_identity_expression(&canonical_property)
            .or_else(|| self.resolve_symbol_identity_expression(property));
        if object_binding.runtime_symbol_properties && requested_symbol.is_some() {
            return None;
        }
        if let Some(value) = object_binding_lookup_value(object_binding, &canonical_property) {
            return Some(value.clone());
        }

        let requested_symbol = requested_symbol?;
        object_binding
            .symbol_properties
            .iter()
            .find_map(|(existing_key, value)| {
                let canonical_existing = self
                    .resolve_symbol_identity_expression(existing_key)
                    .unwrap_or_else(|| existing_key.clone());
                (static_expression_matches(&canonical_existing, &requested_symbol)
                    || static_expression_matches(existing_key, &requested_symbol))
                .then(|| value.clone())
            })
    }

    pub(in crate::backend::direct_wasm) fn resolve_object_binding_property_value_for_object(
        &self,
        object: &Expression,
        object_binding: &ObjectValueBinding,
        property: &Expression,
    ) -> Option<Expression> {
        if let Some(value) = self.resolve_static_builtin_prototype_property_value(object, property)
        {
            return Some(value);
        }
        self.resolve_object_binding_property_value(object_binding, property)
            .or_else(|| {
                self.resolve_object_binding_constructor_capture_symbol_value(
                    object,
                    object_binding,
                    property,
                )
            })
    }

    pub(in crate::backend::direct_wasm) fn resolve_primitive_prototype_property_value(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<Expression> {
        let constructor_name = self.static_primitive_prototype_owner_name(object)?;
        let property = self.canonical_object_property_expression(property);
        let prototype_binding = self.resolve_function_prototype_object_binding(constructor_name)?;
        self.resolve_object_binding_property_value(&prototype_binding, &property)
    }

    fn static_primitive_prototype_owner_name(&self, object: &Expression) -> Option<&'static str> {
        match object {
            Expression::Number(_) => Some("Number"),
            Expression::String(_) => Some("String"),
            Expression::Bool(_) => Some("Boolean"),
            Expression::BigInt(_) => Some("BigInt"),
            Expression::Identifier(name)
                if self.lookup_identifier_kind(name) == Some(StaticValueKind::Symbol) =>
            {
                Some("Symbol")
            }
            Expression::Call { callee, arguments }
                if arguments.is_empty()
                    && matches!(callee.as_ref(), Expression::Identifier(name) if name == "Symbol") =>
            {
                Some("Symbol")
            }
            _ => {
                let materialized = self.materialize_static_expression(object);
                if static_expression_matches(&materialized, object) {
                    None
                } else {
                    self.static_primitive_prototype_owner_name(&materialized)
                }
            }
        }
    }

    fn resolve_static_builtin_prototype_property_value(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<Expression> {
        let property_name = self.static_builtin_prototype_property_name(property)?;
        let owner_name = Self::static_builtin_prototype_owner_name(object)
            .map(str::to_string)
            .or_else(|| {
                let materialized = self.materialize_static_expression(object);
                Self::static_builtin_prototype_owner_name(&materialized).map(str::to_string)
            })?;

        match property_name.as_str() {
            "constructor" => Some(Expression::Identifier(owner_name)),
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_inherited_object_property_value(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<Expression> {
        let trace_inherited_values = std::env::var_os("AYY_TRACE_INHERITED_VALUES").is_some();
        let mut prototype = self.resolve_static_object_prototype_expression(object)?;
        for _ in 0..32 {
            let materialized_prototype = self.materialize_static_expression(&prototype);
            if trace_inherited_values {
                eprintln!(
                    "inherited_value:step object={object:?} property={property:?} prototype={prototype:?} materialized={materialized_prototype:?}"
                );
            }
            if matches!(materialized_prototype, Expression::Null) {
                return None;
            }

            for candidate in [&prototype, &materialized_prototype] {
                if let Some(object_binding) = self.resolve_object_binding_from_expression(candidate)
                    && let Some(value) =
                        self.resolve_object_binding_property_value(&object_binding, property)
                {
                    if trace_inherited_values {
                        eprintln!(
                            "inherited_value:hit candidate={candidate:?} property={property:?} value={value:?}"
                        );
                    }
                    return Some(value);
                }
                if let Some(value) =
                    self.resolve_static_builtin_prototype_property_value(candidate, property)
                {
                    if trace_inherited_values {
                        eprintln!(
                            "inherited_value:static_builtin candidate={candidate:?} property={property:?} value={value:?}"
                        );
                    }
                    return Some(value);
                }
            }

            let next_prototype = self
                .resolve_static_object_prototype_expression(&materialized_prototype)
                .or_else(|| self.resolve_static_object_prototype_expression(&prototype))?;
            if static_expression_matches(&next_prototype, &prototype)
                || static_expression_matches(&next_prototype, &materialized_prototype)
            {
                return None;
            }
            prototype = next_prototype;
        }
        None
    }

    fn static_builtin_prototype_owner_name(expression: &Expression) -> Option<&str> {
        let Expression::Member { object, property } = expression else {
            return None;
        };
        let Expression::Identifier(owner_name) = object.as_ref() else {
            return None;
        };
        if matches!(property.as_ref(), Expression::String(property_name) if property_name == "prototype")
        {
            return Some(owner_name);
        }
        None
    }

    fn static_builtin_prototype_property_name(&self, property: &Expression) -> Option<String> {
        let canonical_property = self.canonical_object_property_expression(property);
        static_property_name_from_expression(&canonical_property)
            .or_else(|| static_property_name_from_expression(property))
    }

    fn static_builtin_prototype_owner_has_own_property(
        owner_name: &str,
        property_name: &str,
    ) -> bool {
        if property_name == "constructor"
            && builtin_constructor_prototype_kind(owner_name).is_some()
        {
            return true;
        }

        if builtin_prototype_function_name(owner_name, property_name).is_some() {
            return true;
        }

        match owner_name {
            "Object" => matches!(
                property_name,
                "constructor"
                    | "__defineGetter__"
                    | "__defineSetter__"
                    | "isPrototypeOf"
                    | "toLocaleString"
                    | "valueOf"
            ),
            "Function" => matches!(property_name, "constructor" | "length" | "name"),
            "Array" => matches!(property_name, "constructor" | "length"),
            _ => false,
        }
    }

    pub(in crate::backend::direct_wasm) fn static_builtin_prototype_has_own_property(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> bool {
        let Some(property_name) = self.static_builtin_prototype_property_name(property) else {
            return false;
        };

        if let Some(owner_name) = Self::static_builtin_prototype_owner_name(object)
            && Self::static_builtin_prototype_owner_has_own_property(owner_name, &property_name)
        {
            return true;
        }

        let materialized_object = self.materialize_static_expression(object);
        if static_expression_matches(&materialized_object, object) {
            return false;
        }
        Self::static_builtin_prototype_owner_name(&materialized_object).is_some_and(|owner_name| {
            Self::static_builtin_prototype_owner_has_own_property(owner_name, &property_name)
        })
    }

    pub(in crate::backend::direct_wasm) fn resolve_inherited_object_has_property(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> bool {
        let Some(mut prototype) = self.resolve_static_object_prototype_expression(object) else {
            return false;
        };
        let canonical_property = self.canonical_object_property_expression(property);

        for _ in 0..32 {
            let materialized_prototype = self.materialize_static_expression(&prototype);
            if matches!(materialized_prototype, Expression::Null) {
                return false;
            }

            for candidate in [&prototype, &materialized_prototype] {
                if self.static_builtin_prototype_has_own_property(candidate, property)
                    || self
                        .resolve_object_binding_from_expression(candidate)
                        .is_some_and(|object_binding| {
                            object_binding_has_property(&object_binding, &canonical_property)
                                || object_binding_lookup_descriptor(
                                    &object_binding,
                                    &canonical_property,
                                )
                                .is_some()
                                || self
                                    .resolve_object_binding_property_value(
                                        &object_binding,
                                        &canonical_property,
                                    )
                                    .is_some()
                                || self
                                    .resolve_object_binding_property_value(
                                        &object_binding,
                                        property,
                                    )
                                    .is_some()
                        })
                {
                    return true;
                }
            }

            let Some(next_prototype) = self
                .resolve_static_object_prototype_expression(&materialized_prototype)
                .or_else(|| self.resolve_static_object_prototype_expression(&prototype))
            else {
                return false;
            };
            if static_expression_matches(&next_prototype, &prototype)
                || static_expression_matches(&next_prototype, &materialized_prototype)
            {
                return false;
            }
            prototype = next_prototype;
        }
        false
    }

    pub(in crate::backend::direct_wasm) fn resolve_object_binding_has_property_with_inherited(
        &self,
        object: &Expression,
        object_binding: &ObjectValueBinding,
        property: &Expression,
    ) -> bool {
        let canonical_property = self.canonical_object_property_expression(property);
        object_binding_has_property(object_binding, &canonical_property)
            || object_binding_lookup_descriptor(object_binding, &canonical_property).is_some()
            || self
                .resolve_object_binding_property_value_for_object(
                    object,
                    object_binding,
                    &canonical_property,
                )
                .is_some()
            || self
                .resolve_object_binding_property_value_for_object(object, object_binding, property)
                .is_some()
            || self.resolve_inherited_object_has_property(object, property)
    }

    pub(in crate::backend::direct_wasm) fn resolve_object_binding_property_value_with_inherited(
        &self,
        object: &Expression,
        object_binding: &ObjectValueBinding,
        property: &Expression,
    ) -> Option<Expression> {
        self.resolve_object_binding_property_value_for_object(object, object_binding, property)
            .or_else(|| self.resolve_inherited_object_property_value(object, property))
    }

    pub(in crate::backend::direct_wasm) fn object_binding_string_property_values_with_inherited(
        &self,
        object: &Expression,
        object_binding: &ObjectValueBinding,
    ) -> Vec<(String, Expression)> {
        let mut values = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for (name, value) in &object_binding.string_properties {
            if seen.insert(name.clone()) {
                values.push((name.clone(), value.clone()));
            }
        }

        let Some(mut prototype) = self.resolve_static_object_prototype_expression(object) else {
            return values;
        };
        for _ in 0..32 {
            let materialized_prototype = self.materialize_static_expression(&prototype);
            if matches!(materialized_prototype, Expression::Null) {
                break;
            }

            for candidate in [&prototype, &materialized_prototype] {
                let Some(prototype_binding) =
                    self.resolve_object_binding_from_expression(candidate)
                else {
                    continue;
                };
                for (name, value) in &prototype_binding.string_properties {
                    if seen.insert(name.clone()) {
                        values.push((name.clone(), value.clone()));
                    }
                }
                break;
            }

            let Some(next_prototype) = self
                .resolve_static_object_prototype_expression(&materialized_prototype)
                .or_else(|| self.resolve_static_object_prototype_expression(&prototype))
            else {
                break;
            };
            if static_expression_matches(&next_prototype, &prototype)
                || static_expression_matches(&next_prototype, &materialized_prototype)
            {
                break;
            }
            prototype = next_prototype;
        }
        values
    }
}
