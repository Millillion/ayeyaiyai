use super::*;

const FAST_STATIC_LOOP_ITERATION_LIMIT: usize = 4096;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn sync_static_resolution_environment_overrides(
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
            resolve_stateful_object_binding_name_in_environment(target, &environment)
                .or_else(|| match target {
                    Expression::Identifier(name) => Some(name.clone()),
                    _ => None,
                })
                .or_else(|| {
                    self.resolve_static_global_object_alias_expression(target)
                        .and_then(|alias| match alias {
                            Expression::Identifier(name) => Some(name),
                            _ => None,
                        })
                })
        else {
            return;
        };
        if !environment.contains_object_binding(&target_name)
            && (target_name == "globalThis"
                || self
                    .resolve_function_binding_from_expression(&Expression::Identifier(
                        target_name.clone(),
                    ))
                    .is_some())
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
            || target_name == "globalThis"
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
            resolve_stateful_object_binding_name_in_environment(object, &environment)
                .or_else(|| match object {
                    Expression::Identifier(name) => Some(name.clone()),
                    Expression::This => Some("this".to_string()),
                    _ => None,
                })
                .or_else(|| {
                    self.resolve_static_global_object_alias_expression(object)
                        .and_then(|alias| match alias {
                            Expression::Identifier(name) => Some(name),
                            _ => None,
                        })
                })
        else {
            return;
        };
        if !environment.contains_object_binding(&target_name)
            && (target_name == "globalThis"
                || self
                    .resolve_function_binding_from_expression(&Expression::Identifier(
                        target_name.clone(),
                    ))
                    .is_some())
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
        } else if target_name == "globalThis" {
            self.backend
                .sync_global_object_binding(&target_name, Some(synced_binding));
            self.backend
                .set_global_binding_kind(&target_name, StaticValueKind::Object);
        }
    }

    pub(in crate::backend::direct_wasm) fn sync_static_executable_statement_tracking_effects_from_environment(
        &mut self,
        statement: &Statement,
        environment: StaticResolutionEnvironment,
    ) -> bool {
        if let Some(environment) =
            self.fast_static_counted_loop_tracking_environment(statement, &environment)
        {
            self.sync_static_resolution_environment_overrides(&environment);
            return true;
        }

        let mut environment = environment;
        if !matches!(
            self.execute_static_statements_with_state(
                std::slice::from_ref(statement),
                &mut environment
            ),
            Some(None)
        ) {
            return false;
        }
        self.sync_static_resolution_environment_overrides(&environment);
        true
    }

    fn fast_static_counted_loop_tracking_environment(
        &self,
        statement: &Statement,
        environment: &StaticResolutionEnvironment,
    ) -> Option<StaticResolutionEnvironment> {
        let Statement::For {
            labels,
            init,
            per_iteration_bindings,
            condition: Some(condition),
            update: Some(update),
            break_hook,
            body,
        } = statement
        else {
            return None;
        };
        if !labels.is_empty() || !per_iteration_bindings.is_empty() || break_hook.is_some() {
            return None;
        }

        let (loop_name, initial_value) = Self::fast_static_loop_init_binding(init)?;
        let Expression::Update {
            name: update_name,
            op: update_op,
            ..
        } = update
        else {
            return None;
        };
        if update_name != loop_name {
            return None;
        }

        let mut environment = environment.clone();
        let initial_value =
            self.fast_static_loop_expression(initial_value, &mut environment, &mut HashMap::new())?;
        if !matches!(initial_value, Expression::Number(_)) {
            return None;
        }
        environment.set_local_binding(loop_name.to_string(), initial_value);

        let mut array_cache = HashMap::new();
        for _ in 0..FAST_STATIC_LOOP_ITERATION_LIMIT {
            match self.fast_static_loop_expression(condition, &mut environment, &mut array_cache)? {
                Expression::Bool(true) => {}
                Expression::Bool(false) => return Some(environment),
                _ => return None,
            }

            if !self.fast_static_loop_execute_block(body, &mut environment, &mut array_cache)? {
                return None;
            }

            let current = environment.binding(loop_name)?;
            let Expression::Number(current) = current else {
                return None;
            };
            let next = match update_op {
                UpdateOp::Increment => current + 1.0,
                UpdateOp::Decrement => current - 1.0,
            };
            environment.assign_binding_value(loop_name.to_string(), Expression::Number(next));
        }

        None
    }

    fn fast_static_loop_init_binding(init: &[Statement]) -> Option<(&str, &Expression)> {
        let [statement] = init else {
            return None;
        };
        match statement {
            Statement::Var { name, value }
            | Statement::Let { name, value, .. }
            | Statement::Assign { name, value } => Some((name.as_str(), value)),
            _ => None,
        }
    }

    fn fast_static_loop_execute_block(
        &self,
        statements: &[Statement],
        environment: &mut StaticResolutionEnvironment,
        array_cache: &mut HashMap<String, ArrayValueBinding>,
    ) -> Option<bool> {
        for statement in statements {
            match statement {
                Statement::Declaration { body }
                | Statement::Block { body }
                | Statement::Labeled { body, .. } => {
                    if !self.fast_static_loop_execute_block(body, environment, array_cache)? {
                        return Some(false);
                    }
                }
                Statement::If {
                    condition,
                    then_branch,
                    else_branch,
                } => {
                    let condition =
                        self.fast_static_loop_expression(condition, environment, array_cache)?;
                    let branch = match condition {
                        Expression::Bool(true) => then_branch,
                        Expression::Bool(false) => else_branch,
                        _ => return None,
                    };
                    if !self.fast_static_loop_execute_block(branch, environment, array_cache)? {
                        return Some(false);
                    }
                }
                Statement::Var { name, value } | Statement::Let { name, value, .. } => {
                    let value =
                        self.fast_static_loop_expression(value, environment, array_cache)?;
                    environment.set_local_binding(name.clone(), value);
                }
                Statement::Assign { name, value } => {
                    let value =
                        self.fast_static_loop_expression(value, environment, array_cache)?;
                    environment.assign_binding_value(name.clone(), value);
                }
                Statement::Expression(expression) => {
                    self.fast_static_loop_expression(expression, environment, array_cache)?;
                }
                Statement::Throw(_) => return Some(false),
                _ => return None,
            }
        }

        Some(true)
    }

    fn fast_static_loop_expression(
        &self,
        expression: &Expression,
        environment: &mut StaticResolutionEnvironment,
        array_cache: &mut HashMap<String, ArrayValueBinding>,
    ) -> Option<Expression> {
        match expression {
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined => Some(expression.clone()),
            Expression::Identifier(name) => environment.binding(name).cloned(),
            Expression::Member { object, property } => {
                self.fast_static_loop_member_expression(object, property, environment, array_cache)
            }
            Expression::Unary { op, expression } => {
                let value =
                    self.fast_static_loop_expression(expression, environment, array_cache)?;
                self.fast_static_loop_unary_expression(*op, value)
            }
            Expression::Binary { op, left, right } => {
                let left = self.fast_static_loop_expression(left, environment, array_cache)?;
                let right = self.fast_static_loop_expression(right, environment, array_cache)?;
                Self::fast_static_loop_binary_expression(*op, left, right)
            }
            Expression::Update { name, op, prefix } => {
                let current = environment.binding(name)?;
                let Expression::Number(current) = current else {
                    return None;
                };
                let current = *current;
                let next = match op {
                    UpdateOp::Increment => current + 1.0,
                    UpdateOp::Decrement => current - 1.0,
                };
                environment.assign_binding_value(name.clone(), Expression::Number(next));
                Some(if *prefix {
                    Expression::Number(next)
                } else {
                    Expression::Number(current)
                })
            }
            Expression::Sequence(expressions) => {
                let mut last = Expression::Undefined;
                for expression in expressions {
                    last =
                        self.fast_static_loop_expression(expression, environment, array_cache)?;
                }
                Some(last)
            }
            _ => None,
        }
    }

    fn fast_static_loop_member_expression(
        &self,
        object: &Expression,
        property: &Expression,
        environment: &mut StaticResolutionEnvironment,
        array_cache: &mut HashMap<String, ArrayValueBinding>,
    ) -> Option<Expression> {
        let property = self.fast_static_loop_expression(property, environment, array_cache)?;
        let Expression::Identifier(object_name) = object else {
            return None;
        };

        if !array_cache.contains_key(object_name) {
            let binding = environment
                .object_binding(object_name)
                .and_then(array_binding_from_object_binding)?;
            array_cache.insert(object_name.clone(), binding);
        }
        let array_binding = array_cache.get(object_name)?;

        if matches!(&property, Expression::String(name) if name == "length") {
            return Some(Expression::Number(array_binding.values.len() as f64));
        }
        let index = argument_index_from_expression(&property)? as usize;
        match array_binding.values.get(index).cloned() {
            Some(Some(value)) => self
                .fast_static_loop_expression(&value, environment, array_cache)
                .or(Some(value)),
            _ => Some(Expression::Undefined),
        }
    }

    fn fast_static_loop_unary_expression(
        &self,
        op: UnaryOp,
        value: Expression,
    ) -> Option<Expression> {
        match op {
            UnaryOp::Plus => {
                Self::fast_static_loop_primitive_to_number(&value).map(Expression::Number)
            }
            UnaryOp::Negate => Self::fast_static_loop_primitive_to_number(&value)
                .map(|value| Expression::Number(-value)),
            UnaryOp::Not => {
                Self::fast_static_loop_truthy(&value).map(|truthy| Expression::Bool(!truthy))
            }
            _ => None,
        }
    }

    fn fast_static_loop_binary_expression(
        op: BinaryOp,
        left: Expression,
        right: Expression,
    ) -> Option<Expression> {
        match op {
            BinaryOp::Add => {
                if matches!(left, Expression::String(_)) || matches!(right, Expression::String(_)) {
                    let left = Self::fast_static_loop_primitive_to_string(&left)?;
                    let right = Self::fast_static_loop_primitive_to_string(&right)?;
                    Some(Expression::String(format!("{left}{right}")))
                } else {
                    let left = Self::fast_static_loop_primitive_to_number(&left)?;
                    let right = Self::fast_static_loop_primitive_to_number(&right)?;
                    Some(Expression::Number(left + right))
                }
            }
            BinaryOp::Subtract => {
                let left = Self::fast_static_loop_primitive_to_number(&left)?;
                let right = Self::fast_static_loop_primitive_to_number(&right)?;
                Some(Expression::Number(left - right))
            }
            BinaryOp::Multiply => {
                let left = Self::fast_static_loop_primitive_to_number(&left)?;
                let right = Self::fast_static_loop_primitive_to_number(&right)?;
                Some(Expression::Number(left * right))
            }
            BinaryOp::Divide => {
                let left = Self::fast_static_loop_primitive_to_number(&left)?;
                let right = Self::fast_static_loop_primitive_to_number(&right)?;
                Some(Expression::Number(left / right))
            }
            BinaryOp::Equal
            | BinaryOp::LooseEqual
            | BinaryOp::NotEqual
            | BinaryOp::LooseNotEqual => {
                let equal = Self::fast_static_loop_equal(&left, &right, op)?;
                Some(Expression::Bool(match op {
                    BinaryOp::Equal | BinaryOp::LooseEqual => equal,
                    BinaryOp::NotEqual | BinaryOp::LooseNotEqual => !equal,
                    _ => unreachable!("equality operator filtered above"),
                }))
            }
            BinaryOp::LessThan
            | BinaryOp::LessThanOrEqual
            | BinaryOp::GreaterThan
            | BinaryOp::GreaterThanOrEqual => {
                let ordering = match (&left, &right) {
                    (Expression::Number(left), Expression::Number(right)) => {
                        left.partial_cmp(right)?
                    }
                    (Expression::String(left), Expression::String(right)) => left.cmp(right),
                    _ => return None,
                };
                Some(Expression::Bool(match op {
                    BinaryOp::LessThan => ordering == std::cmp::Ordering::Less,
                    BinaryOp::LessThanOrEqual => ordering != std::cmp::Ordering::Greater,
                    BinaryOp::GreaterThan => ordering == std::cmp::Ordering::Greater,
                    BinaryOp::GreaterThanOrEqual => ordering != std::cmp::Ordering::Less,
                    _ => unreachable!("comparison operator filtered above"),
                }))
            }
            BinaryOp::LogicalAnd => {
                if Self::fast_static_loop_truthy(&left)? {
                    Some(right)
                } else {
                    Some(left)
                }
            }
            BinaryOp::LogicalOr => {
                if Self::fast_static_loop_truthy(&left)? {
                    Some(left)
                } else {
                    Some(right)
                }
            }
            _ => None,
        }
    }

    fn fast_static_loop_equal(left: &Expression, right: &Expression, op: BinaryOp) -> Option<bool> {
        Some(match (left, right) {
            (Expression::Bool(left), Expression::Bool(right)) => left == right,
            (Expression::Number(left), Expression::Number(right)) => left == right,
            (Expression::String(left), Expression::String(right)) => left == right,
            (Expression::Null, Expression::Null)
            | (Expression::Undefined, Expression::Undefined) => true,
            (Expression::Null, Expression::Undefined)
            | (Expression::Undefined, Expression::Null)
                if matches!(op, BinaryOp::LooseEqual | BinaryOp::LooseNotEqual) =>
            {
                true
            }
            _ => false,
        })
    }

    fn fast_static_loop_primitive_to_number(expression: &Expression) -> Option<f64> {
        match expression {
            Expression::Number(value) => Some(*value),
            Expression::Bool(value) => Some(if *value { 1.0 } else { 0.0 }),
            Expression::Null => Some(0.0),
            Expression::Undefined => Some(f64::NAN),
            _ => None,
        }
    }

    fn fast_static_loop_primitive_to_string(expression: &Expression) -> Option<String> {
        match expression {
            Expression::String(value) => Some(value.clone()),
            Expression::Number(value) => Some(Self::fast_static_loop_number_to_string(*value)),
            Expression::Bool(value) => Some(value.to_string()),
            Expression::Null => Some("null".to_string()),
            Expression::Undefined => Some("undefined".to_string()),
            Expression::BigInt(value) => Some(value.trim_end_matches('n').to_string()),
            _ => None,
        }
    }

    fn fast_static_loop_number_to_string(value: f64) -> String {
        if value.is_nan() {
            "NaN".to_string()
        } else if value == f64::INFINITY {
            "Infinity".to_string()
        } else if value == f64::NEG_INFINITY {
            "-Infinity".to_string()
        } else if value == 0.0 {
            "0".to_string()
        } else if value.is_finite() && value.fract() == 0.0 {
            (value as i64).to_string()
        } else {
            value.to_string()
        }
    }

    fn fast_static_loop_truthy(expression: &Expression) -> Option<bool> {
        Some(match expression {
            Expression::Bool(value) => *value,
            Expression::Number(value) => *value != 0.0 && !value.is_nan(),
            Expression::String(value) => !value.is_empty(),
            Expression::Null | Expression::Undefined => false,
            _ => return None,
        })
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
        let mut target_names = vec![target_name.clone()];
        if let Some(Expression::Identifier(alias)) =
            self.resolve_static_class_init_local_alias_expression(&target_name)
            && !target_names.contains(&alias)
        {
            target_names.push(alias);
        }
        if let Some(Expression::Identifier(alias)) = self.global_value_binding(&target_name)
            && !target_names.contains(alias)
        {
            target_names.push(alias.clone());
        }
        for target_name in target_names {
            self.backend.sync_global_object_prototype_expression(
                &format!("{target_name}.prototype"),
                Some(prototype_parent.clone()),
            );
        }
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
                    Statement::For { .. } | Statement::While { .. } | Statement::DoWhile { .. } => {
                        let environment = compiler.snapshot_static_resolution_environment();
                        compiler.sync_static_executable_statement_tracking_effects_from_environment(
                            statement,
                            environment,
                        );
                    }
                    _ => {}
                }
                Ok(())
            },
        )
        .expect("static statement tracking sync should not fail");
    }
}
