use super::*;

const FAST_STATIC_LOOP_ITERATION_LIMIT: usize = 4096;
const FAST_STATIC_GRID_POINT_LIMIT: usize = 1_000_000;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn sync_static_resolution_environment_overrides(
        &mut self,
        environment: &StaticResolutionEnvironment,
    ) {
        for (name, value) in &environment.global_value_overrides {
            if crate::ayy_env_flag!("AYY_TRACE_SYNC_OVERRIDES") {
                eprintln!("sync_overrides:global name={name} value={value:?}");
            }
            self.update_static_global_assignment_metadata(name, value);
        }
        for (name, value) in &environment.local_bindings {
            if crate::ayy_env_flag!("AYY_TRACE_SYNC_OVERRIDES") {
                eprintln!("sync_overrides:local name={name} value={value:?}");
            }
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
            if let Some(array_binding) = array_binding_from_object_binding(binding) {
                self.state
                    .speculation
                    .static_semantics
                    .set_local_array_binding(name, array_binding);
            }
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

        self.sync_static_property_key_coercion_side_effects(property);

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

    fn sync_static_property_key_coercion_side_effects(&mut self, property: &Expression) {
        let Some(LocalFunctionBinding::User(function_name)) = self
            .resolve_property_key_expression_with_coercion(property)
            .and_then(|resolved| resolved.coercion)
        else {
            return;
        };
        let Some(user_function) = self.user_function(&function_name).cloned() else {
            return;
        };

        let capture_source_bindings = self
            .static_property_key_coercion_capture_source_bindings(&function_name, &user_function);

        if let Some(mut execution) = self.prepare_static_user_function_execution(
            &function_name,
            &user_function,
            &[],
            property,
            Some(&capture_source_bindings),
            capture_source_bindings.clone(),
            |statement| statement,
        ) && self
            .execute_static_statements_with_state(
                &execution.substituted_body,
                &mut execution.environment,
            )
            .is_some()
        {
            let mut synced_bindings =
                self.collect_user_function_assigned_nonlocal_bindings(&user_function);
            synced_bindings
                .extend(self.collect_user_function_updated_nonlocal_bindings(&user_function));
            synced_bindings
                .extend(self.collect_user_function_call_effect_nonlocal_bindings(&user_function));
            for name in synced_bindings {
                let Some(value) = execution.environment.binding(&name).cloned() else {
                    continue;
                };
                if self
                    .sync_bound_capture_source_binding_metadata(&name, &value)
                    .is_err()
                {
                    let mut invalidated = HashSet::new();
                    invalidated.insert(name);
                    self.invalidate_static_binding_metadata_for_names(&invalidated);
                }
            }
            return;
        }

        let mut invalidated_bindings =
            self.collect_user_function_assigned_nonlocal_bindings(&user_function);
        invalidated_bindings
            .extend(self.collect_user_function_updated_nonlocal_bindings(&user_function));
        invalidated_bindings
            .extend(self.collect_user_function_call_effect_nonlocal_bindings(&user_function));
        self.invalidate_static_binding_metadata_for_names(&invalidated_bindings);
    }

    fn static_property_key_coercion_capture_source_bindings(
        &self,
        function_name: &str,
        user_function: &UserFunction,
    ) -> HashMap<String, Expression> {
        let mut bindings = HashMap::new();

        if let Some(captures) = self.user_function_capture_bindings(function_name) {
            for (source_name, hidden_name) in captures {
                bindings.insert(
                    source_name.clone(),
                    self.static_property_key_coercion_capture_source_expression(
                        &source_name,
                        &hidden_name,
                    ),
                );
            }
        }

        let mut source_names = self.collect_user_function_assigned_nonlocal_bindings(user_function);
        source_names.extend(self.collect_user_function_updated_nonlocal_bindings(user_function));
        source_names
            .extend(self.collect_user_function_call_effect_nonlocal_bindings(user_function));
        if let Some(function) = self.resolve_registered_function_declaration(function_name) {
            source_names.extend(collect_referenced_binding_names_from_statements(
                &function.body,
            ));
            for parameter in &function.params {
                if let Some(default) = &parameter.default {
                    collect_referenced_binding_names_from_expression(default, &mut source_names);
                }
            }
        }

        for source_name in source_names {
            if bindings.contains_key(&source_name)
                || source_name == "arguments"
                || user_function
                    .params
                    .iter()
                    .any(|param| param == &source_name)
                || user_function.scope_bindings.contains(&source_name)
            {
                continue;
            }
            let source_expression = self
                .static_property_key_coercion_capture_source_expression(&source_name, &source_name);
            if matches!(&source_expression, Expression::Identifier(name) if name == &source_name) {
                continue;
            }
            bindings.insert(source_name, source_expression);
        }

        bindings
    }

    fn static_property_key_coercion_capture_source_expression(
        &self,
        source_name: &str,
        hidden_name: &str,
    ) -> Expression {
        if source_name == "this" {
            return Expression::This;
        }
        if source_name == "new.target" {
            return Expression::NewTarget;
        }

        let source_identifier = Expression::Identifier(source_name.to_string());
        let hidden_identifier = Expression::Identifier(hidden_name.to_string());

        if let Some(value) = self
            .state
            .speculation
            .static_semantics
            .local_value_binding(source_name)
            .filter(|value| !static_expression_matches(value, &source_identifier))
        {
            return value.clone();
        }

        if let Some(value) = self
            .global_value_binding(source_name)
            .filter(|value| !static_expression_matches(value, &source_identifier))
        {
            return value.clone();
        }

        if let Some(array_binding) = self
            .state
            .speculation
            .static_semantics
            .local_array_binding(source_name)
            .or_else(|| self.global_array_binding(source_name))
        {
            return Self::static_property_key_coercion_array_expression(array_binding);
        }

        if let Some(object_binding) = self
            .state
            .speculation
            .static_semantics
            .local_object_binding(source_name)
            .or_else(|| self.global_object_binding(source_name))
            .filter(|binding| binding.property_descriptors.is_empty())
        {
            return object_binding_to_expression(object_binding);
        }

        if let Some(value) = self
            .global_value_binding(hidden_name)
            .filter(|value| !static_expression_matches(value, &hidden_identifier))
        {
            return value.clone();
        }

        if let Some(array_binding) = self.global_array_binding(hidden_name) {
            return Self::static_property_key_coercion_array_expression(array_binding);
        }

        if let Some(object_binding) = self
            .global_object_binding(hidden_name)
            .filter(|binding| binding.property_descriptors.is_empty())
        {
            return object_binding_to_expression(object_binding);
        }

        if let Some(alias) = self
            .resolve_bound_alias_expression(&source_identifier)
            .filter(|alias| !static_expression_matches(alias, &source_identifier))
        {
            return self.materialize_static_expression(&alias);
        }

        source_identifier
    }

    fn static_property_key_coercion_array_expression(binding: &ArrayValueBinding) -> Expression {
        Expression::Array(
            binding
                .values
                .iter()
                .map(|value| {
                    ArrayElement::Expression(value.clone().unwrap_or(Expression::Undefined))
                })
                .collect(),
        )
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
        let trace_static_exec = crate::ayy_env_flag!("AYY_TRACE_STATIC_EXEC");
        if trace_static_exec {
            eprintln!("static_exec_sync:start statement={statement:?}");
        }
        // The static statement executor resolves identifiers lexically and
        // does not consult active `with` scope objects, so eliding a loop (or
        // syncing its effects) inside a with-scope would read and write the
        // outer bindings instead of the scope object's properties.
        if !self.state.emission.lexical_scopes.with_scopes.is_empty() {
            if trace_static_exec {
                eprintln!("static_exec_sync:skip_with_scope");
            }
            return false;
        }
        if let Some(environment) =
            self.fast_static_counted_loop_tracking_environment(statement, &environment)
        {
            self.sync_static_resolution_environment_overrides(&environment);
            if trace_static_exec {
                eprintln!("static_exec_sync:success_fast");
            }
            return true;
        }
        if let Some(environment) =
            self.fast_static_while_loop_tracking_environment(statement, &environment)
        {
            self.sync_static_resolution_environment_overrides(&environment);
            if trace_static_exec {
                eprintln!("static_exec_sync:success_fast_while");
            }
            return true;
        }

        if matches!(
            statement,
            Statement::For { .. } | Statement::While { .. } | Statement::DoWhile { .. }
        ) {
            if trace_static_exec {
                eprintln!("static_exec_sync:skip_loop_without_fast_path");
            }
            return false;
        }

        let mut environment = environment;
        if !matches!(
            self.execute_static_statements_with_state(
                std::slice::from_ref(statement),
                &mut environment
            ),
            Some(None)
        ) {
            if trace_static_exec {
                eprintln!("static_exec_sync:failed_statement_execution");
            }
            return false;
        }
        self.sync_static_resolution_environment_overrides(&environment);
        if trace_static_exec {
            eprintln!("static_exec_sync:success_shared");
        }
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

    fn fast_static_while_loop_tracking_environment(
        &self,
        statement: &Statement,
        environment: &StaticResolutionEnvironment,
    ) -> Option<StaticResolutionEnvironment> {
        if let Some(environment) =
            self.fast_static_quarter_circle_grid_loop_tracking_environment(statement, environment)
        {
            return Some(environment);
        }

        let Statement::While {
            labels,
            condition,
            break_hook,
            body,
        } = statement
        else {
            return None;
        };
        if !labels.is_empty() || break_hook.is_some() {
            return None;
        }

        let mut environment = environment.clone();
        let mut array_cache = HashMap::new();
        self.fast_static_loop_execute_while(condition, body, &mut environment, &mut array_cache)?;
        Some(environment)
    }

    fn fast_static_quarter_circle_grid_loop_tracking_environment(
        &self,
        statement: &Statement,
        environment: &StaticResolutionEnvironment,
    ) -> Option<StaticResolutionEnvironment> {
        let Statement::While {
            labels,
            condition,
            break_hook,
            body,
        } = statement
        else {
            return None;
        };
        if !labels.is_empty() || break_hook.is_some() {
            return None;
        }

        let (outer_index_name, divisions_name) =
            Self::static_loop_less_than_or_equal_identifier_bound(condition)?;
        let outer_body = Self::static_flatten_single_block(body);
        let [inner_index_init, inner_loop, outer_increment] = outer_body else {
            return None;
        };
        let (inner_index_name, inner_start) = Self::static_loop_var_number_init(inner_index_init)?;
        if inner_start != 0.0
            || !Self::static_loop_assigns_increment_by_one(outer_increment, outer_index_name)
        {
            return None;
        }

        let Statement::While {
            labels,
            condition,
            break_hook,
            body,
        } = inner_loop
        else {
            return None;
        };
        if !labels.is_empty() || break_hook.is_some() {
            return None;
        }
        let (inner_condition_name, inner_divisions_name) =
            Self::static_loop_less_than_or_equal_identifier_bound(condition)?;
        if inner_condition_name != inner_index_name || inner_divisions_name != divisions_name {
            return None;
        }

        let inner_body = Self::static_flatten_single_block(body);
        let [
            x_assignment,
            y_assignment,
            inside_if,
            total_increment,
            inner_increment,
        ] = inner_body
        else {
            return None;
        };
        let x_name = Self::static_loop_var_division_by_identifier(
            x_assignment,
            outer_index_name,
            divisions_name,
        )?;
        let y_name = Self::static_loop_var_division_by_identifier(
            y_assignment,
            inner_index_name,
            divisions_name,
        )?;
        let inside_name =
            Self::static_quarter_circle_inside_increment_name(inside_if, x_name, y_name)?;
        let total_name = Self::static_loop_increment_assignment_name(total_increment)?;
        if !Self::static_loop_assigns_increment_by_one(inner_increment, inner_index_name) {
            return None;
        }

        let divisions = Self::static_loop_environment_number(environment, divisions_name)?;
        let outer_start = Self::static_loop_environment_number(environment, outer_index_name)?;
        let inside_start = Self::static_loop_environment_number(environment, inside_name)?;
        let total_start = Self::static_loop_environment_number(environment, total_name)?;
        let divisions_count = Self::static_loop_non_negative_integer(divisions)?;
        let outer_start_count = Self::static_loop_non_negative_integer(outer_start)?;
        if outer_start_count > divisions_count {
            return None;
        }
        let outer_points = divisions_count - outer_start_count + 1;
        let inner_points = divisions_count + 1;
        if outer_points.saturating_mul(inner_points) > FAST_STATIC_GRID_POINT_LIMIT {
            return None;
        }

        let mut inside = inside_start;
        let mut total = total_start;
        let mut last_x = 0.0;
        let mut last_y = 0.0;
        for x_index in outer_start_count..=divisions_count {
            let x = x_index as f64 / divisions;
            for y_index in 0..=divisions_count {
                let y = y_index as f64 / divisions;
                if x * x + y * y <= 1.0 {
                    inside += 1.0;
                }
                total += 1.0;
                last_x = x;
                last_y = y;
            }
        }

        let mut environment = environment.clone();
        environment.assign_binding_value(
            outer_index_name.to_string(),
            Expression::Number((divisions_count + 1) as f64),
        );
        environment.assign_binding_value(
            inner_index_name.to_string(),
            Expression::Number((divisions_count + 1) as f64),
        );
        environment.assign_binding_value(x_name.to_string(), Expression::Number(last_x));
        environment.assign_binding_value(y_name.to_string(), Expression::Number(last_y));
        environment.assign_binding_value(inside_name.to_string(), Expression::Number(inside));
        environment.assign_binding_value(total_name.to_string(), Expression::Number(total));
        Some(environment)
    }

    fn static_flatten_single_block(statements: &[Statement]) -> &[Statement] {
        if let [Statement::Block { body }] = statements {
            body
        } else {
            statements
        }
    }

    fn static_loop_environment_number(
        environment: &StaticResolutionEnvironment,
        name: &str,
    ) -> Option<f64> {
        let Expression::Number(value) = environment.binding(name)? else {
            return None;
        };
        Some(*value)
    }

    fn static_loop_non_negative_integer(value: f64) -> Option<usize> {
        (value.is_finite() && value >= 0.0 && value.fract() == 0.0).then_some(value as usize)
    }

    fn static_loop_less_than_or_equal_identifier_bound(
        expression: &Expression,
    ) -> Option<(&str, &str)> {
        let Expression::Binary {
            op: BinaryOp::LessThanOrEqual,
            left,
            right,
        } = expression
        else {
            return None;
        };
        let Expression::Identifier(left) = left.as_ref() else {
            return None;
        };
        let Expression::Identifier(right) = right.as_ref() else {
            return None;
        };
        Some((left, right))
    }

    fn static_loop_var_number_init(statement: &Statement) -> Option<(&str, f64)> {
        match statement {
            Statement::Var { name, value } | Statement::Let { name, value, .. } => {
                let Expression::Number(value) = value else {
                    return None;
                };
                Some((name, *value))
            }
            _ => None,
        }
    }

    fn static_loop_var_division_by_identifier<'b>(
        statement: &'b Statement,
        numerator_name: &str,
        denominator_name: &str,
    ) -> Option<&'b str> {
        let (name, value) = match statement {
            Statement::Var { name, value } | Statement::Let { name, value, .. } => {
                (name.as_str(), value)
            }
            _ => return None,
        };
        if Self::static_expression_is_identifier_division(value, numerator_name, denominator_name) {
            Some(name)
        } else {
            None
        }
    }

    fn static_expression_is_identifier_division(
        expression: &Expression,
        numerator_name: &str,
        denominator_name: &str,
    ) -> bool {
        matches!(
            expression,
            Expression::Binary {
                op: BinaryOp::Divide,
                left,
                right,
            } if matches!(left.as_ref(), Expression::Identifier(name) if name == numerator_name)
                && matches!(right.as_ref(), Expression::Identifier(name) if name == denominator_name)
        )
    }

    fn static_quarter_circle_inside_increment_name<'b>(
        statement: &'b Statement,
        x_name: &str,
        y_name: &str,
    ) -> Option<&'b str> {
        let Statement::If {
            condition,
            then_branch,
            else_branch,
        } = statement
        else {
            return None;
        };
        if !else_branch.is_empty()
            || !Self::static_expression_is_quarter_circle_condition(condition, x_name, y_name)
        {
            return None;
        }
        let [statement] = Self::static_flatten_single_block(then_branch) else {
            return None;
        };
        Self::static_loop_increment_assignment_name(statement)
    }

    fn static_expression_is_quarter_circle_condition(
        expression: &Expression,
        x_name: &str,
        y_name: &str,
    ) -> bool {
        let Expression::Binary {
            op: BinaryOp::LessThanOrEqual,
            left,
            right,
        } = expression
        else {
            return false;
        };
        if !matches!(right.as_ref(), Expression::Number(value) if *value == 1.0) {
            return false;
        }
        let Expression::Binary {
            op: BinaryOp::Add,
            left,
            right,
        } = left.as_ref()
        else {
            return false;
        };
        Self::static_expression_is_identifier_square(left, x_name)
            && Self::static_expression_is_identifier_square(right, y_name)
    }

    fn static_expression_is_identifier_square(expression: &Expression, name: &str) -> bool {
        matches!(
            expression,
            Expression::Binary {
                op: BinaryOp::Multiply,
                left,
                right,
            } if matches!(left.as_ref(), Expression::Identifier(left) if left == name)
                && matches!(right.as_ref(), Expression::Identifier(right) if right == name)
        )
    }

    fn static_loop_increment_assignment_name(statement: &Statement) -> Option<&str> {
        let Statement::Assign { name, value } = statement else {
            return None;
        };
        if Self::static_expression_is_identifier_plus_one(value, name) {
            Some(name)
        } else {
            None
        }
    }

    fn static_loop_assigns_increment_by_one(statement: &Statement, name: &str) -> bool {
        matches!(
            statement,
            Statement::Assign { name: assigned_name, value }
                if assigned_name == name && Self::static_expression_is_identifier_plus_one(value, name)
        )
    }

    fn static_expression_is_identifier_plus_one(expression: &Expression, name: &str) -> bool {
        matches!(
            expression,
            Expression::Binary {
                op: BinaryOp::Add,
                left,
                right,
            } if matches!(left.as_ref(), Expression::Identifier(left) if left == name)
                && matches!(right.as_ref(), Expression::Number(value) if *value == 1.0)
        )
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
                Statement::AssignMember {
                    object,
                    property,
                    value,
                } => {
                    self.fast_static_loop_assign_member_expression(
                        object,
                        property,
                        value,
                        environment,
                        array_cache,
                    )?;
                }
                Statement::Expression(expression) => {
                    self.fast_static_loop_expression(expression, environment, array_cache)?;
                }
                Statement::Throw(_) => return Some(false),
                Statement::While {
                    labels,
                    condition,
                    break_hook,
                    body,
                } => {
                    if !labels.is_empty() || break_hook.is_some() {
                        return None;
                    }
                    self.fast_static_loop_execute_while(condition, body, environment, array_cache)?;
                }
                _ => return None,
            }
        }

        Some(true)
    }

    fn fast_static_loop_execute_while(
        &self,
        condition: &Expression,
        body: &[Statement],
        environment: &mut StaticResolutionEnvironment,
        array_cache: &mut HashMap<String, ArrayValueBinding>,
    ) -> Option<bool> {
        for _ in 0..FAST_STATIC_LOOP_ITERATION_LIMIT {
            match self.fast_static_loop_expression(condition, environment, array_cache)? {
                Expression::Bool(true) => {}
                Expression::Bool(false) => return Some(true),
                _ => return None,
            }

            if !self.fast_static_loop_execute_block(body, environment, array_cache)? {
                return Some(false);
            }
        }

        None
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
            Expression::Identifier(name) => {
                if let Some(value) = environment.binding(name).cloned() {
                    if matches!(value, Expression::Object(_) | Expression::Array(_))
                        && environment.contains_object_binding(name)
                    {
                        return Some(expression.clone());
                    }
                    return self
                        .fast_static_loop_expression(&value, environment, array_cache)
                        .or(Some(value));
                }
                environment
                    .contains_object_binding(name)
                    .then(|| expression.clone())
            }
            Expression::Assign { name, value } => {
                let value = self.fast_static_loop_expression(value, environment, array_cache)?;
                environment.assign_binding_value(name.clone(), value.clone());
                Some(value)
            }
            Expression::Member { object, property } => {
                self.fast_static_loop_member_expression(object, property, environment, array_cache)
            }
            Expression::AssignMember {
                object,
                property,
                value,
            } => self.fast_static_loop_assign_member_expression(
                object,
                property,
                value,
                environment,
                array_cache,
            ),
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
            Expression::Call { callee, arguments } => {
                self.fast_static_loop_call_expression(callee, arguments, environment, array_cache)
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

    fn fast_static_loop_call_expression(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
        environment: &mut StaticResolutionEnvironment,
        array_cache: &mut HashMap<String, ArrayValueBinding>,
    ) -> Option<Expression> {
        let Expression::Identifier(function_name) = callee else {
            return None;
        };
        if !self.function_is_static_linked_list_append_helper(function_name) {
            return None;
        }
        let [
            CallArgument::Expression(state),
            CallArgument::Expression(index),
            CallArgument::Expression(value),
        ] = arguments
        else {
            return None;
        };
        let state_name = match state {
            Expression::Identifier(name) => name.clone(),
            _ => match self.fast_static_loop_expression(state, environment, array_cache)? {
                Expression::Identifier(name) => name,
                _ => return None,
            },
        };
        let index = self.fast_static_loop_expression(index, environment, array_cache)?;
        let value = self.fast_static_loop_expression(value, environment, array_cache)?;
        self.fast_static_loop_append_linked_list_node(&state_name, index, value, environment)?;
        Some(Expression::Undefined)
    }

    fn function_is_static_linked_list_append_helper(&self, function_name: &str) -> bool {
        let Some(declaration) = self.prepared_function_declaration(function_name) else {
            return false;
        };
        if declaration.params.len() != 3 {
            return false;
        }
        let state_param = &declaration.params[0].name;
        let index_param = &declaration.params[1].name;
        let value_param = &declaration.params[2].name;
        let Some(node_name) = Self::static_linked_list_append_node_binding_name(
            &declaration.body,
            index_param,
            value_param,
        ) else {
            return false;
        };
        Self::statements_assign_static_member_property(
            &declaration.body,
            &Expression::Identifier(state_param.clone()),
            "head",
            &Expression::Identifier(node_name.clone()),
        ) && Self::statements_assign_static_member_property(
            &declaration.body,
            &Expression::Identifier(state_param.clone()),
            "tail",
            &Expression::Identifier(node_name.clone()),
        ) && Self::statements_assign_static_member_property(
            &declaration.body,
            &Expression::Member {
                object: Box::new(Expression::Identifier(state_param.clone())),
                property: Box::new(Expression::String("tail".to_string())),
            },
            "next",
            &Expression::Identifier(node_name),
        )
    }

    fn static_linked_list_append_node_binding_name(
        statements: &[Statement],
        index_param: &str,
        value_param: &str,
    ) -> Option<String> {
        for statement in statements {
            match statement {
                Statement::Var { name, value } | Statement::Let { name, value, .. } => {
                    if Self::object_literal_is_static_linked_list_node(
                        value,
                        index_param,
                        value_param,
                    ) {
                        return Some(name.clone());
                    }
                }
                Statement::Declaration { body }
                | Statement::Block { body }
                | Statement::Labeled { body, .. } => {
                    if let Some(name) = Self::static_linked_list_append_node_binding_name(
                        body,
                        index_param,
                        value_param,
                    ) {
                        return Some(name);
                    }
                }
                Statement::If {
                    then_branch,
                    else_branch,
                    ..
                } => {
                    if let Some(name) = Self::static_linked_list_append_node_binding_name(
                        then_branch,
                        index_param,
                        value_param,
                    ) {
                        return Some(name);
                    }
                    if let Some(name) = Self::static_linked_list_append_node_binding_name(
                        else_branch,
                        index_param,
                        value_param,
                    ) {
                        return Some(name);
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn object_literal_is_static_linked_list_node(
        expression: &Expression,
        index_param: &str,
        value_param: &str,
    ) -> bool {
        let Expression::Object(entries) = expression else {
            return false;
        };
        let mut has_index = false;
        let mut has_value = false;
        let mut has_next = false;
        for entry in entries {
            let ObjectEntry::Data { key, value } = entry else {
                return false;
            };
            match (key, value) {
                (Expression::String(name), Expression::Identifier(value_name))
                    if name == "index" && value_name == index_param =>
                {
                    has_index = true;
                }
                (Expression::String(name), Expression::Identifier(value_name))
                    if name == "value" && value_name == value_param =>
                {
                    has_value = true;
                }
                (Expression::String(name), Expression::Null) if name == "next" => {
                    has_next = true;
                }
                _ => return false,
            }
        }
        has_index && has_value && has_next
    }

    fn statements_assign_static_member_property(
        statements: &[Statement],
        object: &Expression,
        property_name: &str,
        value: &Expression,
    ) -> bool {
        statements.iter().any(|statement| {
            Self::statement_assigns_static_member_property(statement, object, property_name, value)
        })
    }

    fn statement_assigns_static_member_property(
        statement: &Statement,
        object: &Expression,
        property_name: &str,
        value: &Expression,
    ) -> bool {
        match statement {
            Statement::AssignMember {
                object: assigned_object,
                property,
                value: assigned_value,
            } => {
                static_expression_matches(assigned_object, object)
                    && matches!(property, Expression::String(name) if name == property_name)
                    && static_expression_matches(assigned_value, value)
            }
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => {
                Self::statements_assign_static_member_property(body, object, property_name, value)
            }
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => {
                Self::statements_assign_static_member_property(
                    then_branch,
                    object,
                    property_name,
                    value,
                ) || Self::statements_assign_static_member_property(
                    else_branch,
                    object,
                    property_name,
                    value,
                )
            }
            _ => false,
        }
    }

    fn fast_static_loop_append_linked_list_node(
        &self,
        state_name: &str,
        index: Expression,
        value: Expression,
        environment: &mut StaticResolutionEnvironment,
    ) -> Option<()> {
        let mut state_binding = environment.object_binding(state_name).cloned()?;
        let node_name = self.fast_static_loop_next_linked_list_node_name(
            state_name,
            &state_binding,
            environment,
        );
        let node_expression = Expression::Identifier(node_name.clone());

        let mut node_binding = empty_object_value_binding();
        object_binding_set_property(
            &mut node_binding,
            Expression::String("index".to_string()),
            index,
        );
        object_binding_set_property(
            &mut node_binding,
            Expression::String("value".to_string()),
            value,
        );
        object_binding_set_property(
            &mut node_binding,
            Expression::String("next".to_string()),
            Expression::Null,
        );

        let head_property = Expression::String("head".to_string());
        let tail_property = Expression::String("tail".to_string());
        let next_property = Expression::String("next".to_string());
        let head = object_binding_lookup_value(&state_binding, &head_property)
            .cloned()
            .unwrap_or(Expression::Null);
        match head {
            Expression::Null | Expression::Undefined => {
                object_binding_set_property(
                    &mut state_binding,
                    head_property,
                    node_expression.clone(),
                );
            }
            _ => {
                let Expression::Identifier(tail_name) =
                    object_binding_lookup_value(&state_binding, &tail_property)?.clone()
                else {
                    return None;
                };
                let mut tail_binding = environment.object_binding(&tail_name).cloned()?;
                object_binding_set_property(
                    &mut tail_binding,
                    next_property,
                    node_expression.clone(),
                );
                environment.set_local_object_binding(tail_name, tail_binding);
            }
        }
        object_binding_set_property(&mut state_binding, tail_property, node_expression);
        environment.set_local_object_binding(node_name, node_binding);
        environment.set_object_binding(state_name.to_string(), state_binding);
        Some(())
    }

    fn fast_static_loop_next_linked_list_node_name(
        &self,
        state_name: &str,
        state_binding: &ObjectValueBinding,
        environment: &StaticResolutionEnvironment,
    ) -> String {
        let mut count = 0usize;
        let mut current =
            object_binding_lookup_value(state_binding, &Expression::String("head".to_string()))
                .cloned();
        while let Some(Expression::Identifier(name)) = current {
            count += 1;
            current = environment.object_binding(&name).and_then(|binding| {
                object_binding_lookup_value(binding, &Expression::String("next".to_string()))
                    .cloned()
            });
            if count > FAST_STATIC_LOOP_ITERATION_LIMIT {
                break;
            }
        }
        format!("__ayy_static_list_node${state_name}${count}")
    }

    fn fast_static_loop_assign_member_expression(
        &self,
        object: &Expression,
        property: &Expression,
        value: &Expression,
        environment: &mut StaticResolutionEnvironment,
        array_cache: &mut HashMap<String, ArrayValueBinding>,
    ) -> Option<Expression> {
        let property = self.fast_static_loop_expression(property, environment, array_cache)?;
        let value = self.fast_static_loop_expression(value, environment, array_cache)?;
        let object = self.fast_static_loop_expression(object, environment, array_cache)?;
        let Expression::Identifier(object_name) = object else {
            return None;
        };
        let index = argument_index_from_expression(&property)? as usize;

        if !array_cache.contains_key(&object_name) {
            let binding = environment
                .object_binding(&object_name)
                .and_then(array_binding_from_object_binding)?;
            array_cache.insert(object_name.clone(), binding);
        }

        let mut array_binding = array_cache.get(&object_name)?.clone();
        if array_binding.values.len() <= index {
            array_binding.values.resize(index + 1, None);
        }
        array_binding.values[index] = Some(value.clone());
        array_cache.insert(object_name.clone(), array_binding.clone());
        environment.sync_object_binding(
            &object_name,
            Some(object_binding_from_array_binding(&array_binding)),
        );
        Some(value)
    }

    fn fast_static_loop_member_expression(
        &self,
        object: &Expression,
        property: &Expression,
        environment: &mut StaticResolutionEnvironment,
        array_cache: &mut HashMap<String, ArrayValueBinding>,
    ) -> Option<Expression> {
        let property = self.fast_static_loop_expression(property, environment, array_cache)?;
        let object = self.fast_static_loop_expression(object, environment, array_cache)?;
        let Expression::Identifier(object_name) = object else {
            return None;
        };

        if let Some(object_binding) = environment.object_binding(&object_name)
            && let Some(value) = object_binding_lookup_value(object_binding, &property).cloned()
        {
            return self
                .fast_static_loop_expression(&value, environment, array_cache)
                .or(Some(value));
        }

        if !array_cache.contains_key(&object_name) {
            let binding = environment
                .object_binding(&object_name)
                .and_then(array_binding_from_object_binding)?;
            array_cache.insert(object_name.clone(), binding);
        }
        let array_binding = array_cache.get(&object_name)?;

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
            BinaryOp::Modulo => {
                let left = Self::fast_static_loop_primitive_to_number(&left)?;
                let right = Self::fast_static_loop_primitive_to_number(&right)?;
                Some(Expression::Number(left % right))
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
        js_number_to_string(value)
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

        self.update_prototype_object_binding_without_snapshot(&target_name, &prototype_object);
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
