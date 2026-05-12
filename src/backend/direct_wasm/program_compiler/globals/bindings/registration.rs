use super::*;

impl DirectWasmCompiler {
    fn identifier_function_value_capture_slots_key(name: &str) -> MemberFunctionBindingKey {
        MemberFunctionBindingKey {
            target: MemberFunctionBindingTarget::Identifier(name.to_string()),
            property: MemberFunctionBindingProperty::String(
                "__ayy[[FunctionValueCaptureSlots]]".to_string(),
            ),
        }
    }

    fn normalize_local_member_binding_identifier_target(&self, name: &str) -> String {
        self.state
            .function_registry
            .catalog
            .registered_function(name)
            .and_then(|function| {
                function
                    .self_binding
                    .as_ref()
                    .or(function.top_level_binding.as_ref())
            })
            .cloned()
            .or_else(|| scoped_binding_source_name(name).map(str::to_string))
            .unwrap_or_else(|| name.to_string())
    }

    fn resolve_local_member_metadata_expression(
        expression: &Expression,
        local_bindings: &HashMap<String, Expression>,
    ) -> Expression {
        match expression {
            Expression::Identifier(name) => local_bindings
                .get(name)
                .cloned()
                .unwrap_or_else(|| expression.clone()),
            Expression::Member { object, property } => Expression::Member {
                object: Box::new(Self::resolve_local_member_metadata_expression(
                    object,
                    local_bindings,
                )),
                property: property.clone(),
            },
            _ => expression.clone(),
        }
    }

    fn local_member_binding_target(
        &self,
        target: &Expression,
        local_bindings: &HashMap<String, Expression>,
    ) -> Option<(MemberFunctionBindingTarget, String)> {
        let resolved_target =
            Self::resolve_local_member_metadata_expression(target, local_bindings);
        match resolved_target {
            Expression::Identifier(name) => Some((
                MemberFunctionBindingTarget::Identifier(
                    self.normalize_local_member_binding_identifier_target(&name),
                ),
                self.normalize_local_member_binding_identifier_target(&name),
            )),
            Expression::Member { object, property } if matches!(property.as_ref(), Expression::String(name) if name == "prototype") =>
            {
                let Expression::Identifier(name) = object.as_ref() else {
                    return None;
                };
                let normalized_name = self.normalize_local_member_binding_identifier_target(name);
                Some((
                    MemberFunctionBindingTarget::Prototype(normalized_name.clone()),
                    format!("{normalized_name}.prototype"),
                ))
            }
            _ => None,
        }
    }

    fn synthetic_member_capture_slots(
        &self,
        function_name: &str,
    ) -> Option<BTreeMap<String, String>> {
        let function = self
            .state
            .function_registry
            .catalog
            .registered_function(function_name)?;
        (!function.synthetic_capture_bindings.is_empty()).then(|| {
            let capture_bindings = self
                .state
                .function_registry
                .analysis
                .user_function_capture_bindings
                .get(function_name);
            function
                .synthetic_capture_bindings
                .iter()
                .cloned()
                .map(|name: String| {
                    let slot_name = capture_bindings
                        .and_then(|bindings| bindings.get(&name).cloned())
                        .unwrap_or_else(|| name.clone());
                    (name, slot_name)
                })
                .collect()
        })
    }

    fn register_local_class_member_binding_from_descriptor(
        &mut self,
        target: &Expression,
        property: &Expression,
        descriptor: &PropertyDescriptorDefinition,
        local_bindings: &HashMap<String, Expression>,
    ) {
        let Some((target, home_object_name)) =
            self.local_member_binding_target(target, local_bindings)
        else {
            return;
        };
        let Some(property) = self.global_member_function_binding_property(property) else {
            return;
        };
        let key = MemberFunctionBindingKey {
            target,
            property,
        };

        if let Some(binding) = descriptor
            .value
            .as_ref()
            .and_then(|expression| self.infer_global_function_binding(expression))
        {
            if let LocalFunctionBinding::User(function_name) = &binding {
                self.update_user_function_home_object_binding(binding.clone(), &home_object_name);
                if let Some(capture_slots) = self.synthetic_member_capture_slots(function_name) {
                    self.set_global_member_function_capture_slots(key.clone(), capture_slots);
                }
            }
            self.set_global_member_function_binding(key.clone(), binding);
        }

        if let Some(binding) = descriptor
            .getter
            .as_ref()
            .and_then(|expression| self.infer_global_function_binding(expression))
        {
            if let LocalFunctionBinding::User(function_name) = &binding {
                self.update_user_function_home_object_binding(binding.clone(), &home_object_name);
                if let Some(capture_slots) = self.synthetic_member_capture_slots(function_name) {
                    self.set_global_member_function_capture_slots(key.clone(), capture_slots);
                }
            }
            self.set_global_member_getter_binding(key.clone(), binding);
        }

        if let Some(binding) = descriptor
            .setter
            .as_ref()
            .and_then(|expression| self.infer_global_function_binding(expression))
        {
            if let LocalFunctionBinding::User(function_name) = &binding {
                self.update_user_function_home_object_binding(binding.clone(), &home_object_name);
                if let Some(capture_slots) = self.synthetic_member_capture_slots(function_name) {
                    self.set_global_member_function_capture_slots(key.clone(), capture_slots);
                }
            }
            self.set_global_member_setter_binding(key, binding);
        }
    }

    fn register_local_class_member_bindings_in_statements(
        &mut self,
        statements: &[Statement],
        local_bindings: &mut HashMap<String, Expression>,
    ) {
        for statement in statements {
            match statement {
                Statement::Var { name, value } | Statement::Let { name, value, .. } => {
                    local_bindings.insert(
                        name.clone(),
                        Self::resolve_local_member_metadata_expression(value, local_bindings),
                    );
                }
                Statement::Assign { name, value } => {
                    local_bindings.insert(
                        name.clone(),
                        Self::resolve_local_member_metadata_expression(value, local_bindings),
                    );
                }
                Statement::Declaration { body }
                | Statement::Block { body }
                | Statement::Labeled { body, .. }
                | Statement::With { body, .. } => {
                    let mut nested_bindings = local_bindings.clone();
                    self.register_local_class_member_bindings_in_statements(
                        body,
                        &mut nested_bindings,
                    );
                }
                Statement::If {
                    then_branch,
                    else_branch,
                    ..
                } => {
                    let mut then_bindings = local_bindings.clone();
                    self.register_local_class_member_bindings_in_statements(
                        then_branch,
                        &mut then_bindings,
                    );
                    let mut else_bindings = local_bindings.clone();
                    self.register_local_class_member_bindings_in_statements(
                        else_branch,
                        &mut else_bindings,
                    );
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
                        CallArgument::Expression(target),
                        CallArgument::Expression(property),
                        CallArgument::Expression(descriptor_expression),
                        ..,
                    ] = arguments.as_slice()
                    else {
                        continue;
                    };
                    let Some(descriptor) =
                        resolve_property_descriptor_definition(descriptor_expression)
                    else {
                        continue;
                    };
                    let property =
                        Self::resolve_local_member_metadata_expression(property, local_bindings);
                    self.register_local_class_member_binding_from_descriptor(
                        target,
                        &property,
                        &descriptor,
                        local_bindings,
                    );
                }
                _ => {}
            }
        }
    }

    pub(in crate::backend::direct_wasm) fn register_local_class_member_bindings(
        &mut self,
        functions: &[FunctionDeclaration],
    ) {
        for function in functions {
            let mut local_bindings = HashMap::new();
            self.register_local_class_member_bindings_in_statements(
                &function.body,
                &mut local_bindings,
            );
        }

        let (value_bindings, _) = self.snapshot_top_level_static_state();
        for (name, value) in value_bindings {
            if std::env::var_os("AYY_TRACE_MEMBER_BINDINGS").is_some() {
                eprintln!("global_member:class_alias_candidate name={name} value={value:?}");
            }
            let source_name = match value {
                Expression::Identifier(source_name) => Some(source_name),
                Expression::Call { callee, arguments } if arguments.is_empty() => {
                    if let Expression::Identifier(function_name) = callee.as_ref() {
                        match self.infer_static_class_init_call_result_expression(function_name) {
                            Some(Expression::Identifier(source_name)) => Some(source_name),
                            _ => None,
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            };
            let Some(source_name) = source_name else {
                continue;
            };
            let source_name = self.normalize_local_member_binding_identifier_target(&source_name);
            if std::env::var_os("AYY_TRACE_MEMBER_BINDINGS").is_some() {
                eprintln!(
                    "global_member:class_alias_resolved name={name} source={source_name}"
                );
            }
            if name == source_name || !source_name.starts_with("__ayy_class_expr_") {
                continue;
            }
            if self.has_global_member_bindings_for_name(&source_name) {
                self.copy_global_member_bindings_for_alias(&name, &source_name);
            }
        }
    }

    fn function_references_nested_function(
        function: &FunctionDeclaration,
        nested_function_name: &str,
    ) -> bool {
        if Self::function_body_references_nested_function(function, nested_function_name) {
            return true;
        }
        function.params.iter().any(|parameter| {
            parameter.default.as_ref().is_some_and(|default| {
                let mut referenced = HashSet::new();
                collect_referenced_binding_names_from_expression(default, &mut referenced);
                referenced.contains(nested_function_name)
            })
        })
    }

    fn function_body_references_nested_function(
        function: &FunctionDeclaration,
        nested_function_name: &str,
    ) -> bool {
        collect_referenced_binding_names_from_statements(&function.body)
            .contains(nested_function_name)
    }

    fn function_parameters_reference_nested_function(
        function: &FunctionDeclaration,
        nested_function_name: &str,
    ) -> bool {
        function.params.iter().any(|parameter| {
            parameter.default.as_ref().is_some_and(|default| {
                let mut referenced = HashSet::new();
                collect_referenced_binding_names_from_expression(default, &mut referenced);
                referenced.contains(nested_function_name)
            })
        })
    }

    fn function_has_local_binding_source(
        function: &FunctionDeclaration,
        source_name: &str,
    ) -> bool {
        collect_function_constructor_local_bindings(function)
            .into_iter()
            .any(|name| scoped_binding_source_name(&name).unwrap_or(&name) == source_name)
    }

    fn function_has_parameter_binding_source(
        function: &FunctionDeclaration,
        source_name: &str,
    ) -> bool {
        source_name == "arguments"
            || function.params.iter().any(|parameter| {
                scoped_binding_source_name(&parameter.name).unwrap_or(&parameter.name)
                    == source_name
            })
            || function
                .self_binding
                .as_deref()
                .is_some_and(|binding| binding == source_name)
    }

    fn nested_function_captures_enclosing_body_local(
        functions: &[FunctionDeclaration],
        function_index: usize,
        source_name: &str,
    ) -> bool {
        let Some(function) = functions.get(function_index) else {
            return false;
        };
        let can_capture_from_later_enclosing_candidate =
            is_internal_user_function_identifier(&function.name);
        functions
            .iter()
            .enumerate()
            .any(|(candidate_index, candidate)| {
                candidate_index != function_index
                    && (candidate_index < function_index
                        || can_capture_from_later_enclosing_candidate)
                    && Self::function_body_references_nested_function(candidate, &function.name)
                    && Self::function_has_local_binding_source(candidate, source_name)
            })
    }

    fn nested_function_captures_enclosing_parameter_local(
        functions: &[FunctionDeclaration],
        function_index: usize,
        source_name: &str,
    ) -> bool {
        let Some(function) = functions.get(function_index) else {
            return false;
        };
        let can_capture_from_later_enclosing_candidate =
            is_internal_user_function_identifier(&function.name);
        functions
            .iter()
            .enumerate()
            .any(|(candidate_index, candidate)| {
                candidate_index != function_index
                    && (candidate_index < function_index
                        || can_capture_from_later_enclosing_candidate)
                    && Self::function_parameters_reference_nested_function(
                        candidate,
                        &function.name,
                    )
                    && Self::function_has_local_binding_source(candidate, source_name)
            })
    }

    fn function_references_lexical_this(function: &FunctionDeclaration) -> bool {
        function.lexical_this
            && (statements_reference_this(&function.body)
                || function.params.iter().any(|parameter| {
                    parameter
                        .default
                        .as_ref()
                        .is_some_and(expression_references_this)
                }))
    }

    fn function_references_lexical_new_target(function: &FunctionDeclaration) -> bool {
        function.lexical_this
            && (function
                .body
                .iter()
                .any(Self::statement_references_new_target)
                || function.params.iter().any(|parameter| {
                    parameter
                        .default
                        .as_ref()
                        .is_some_and(Self::expression_references_new_target)
                }))
    }

    fn statement_references_new_target(statement: &Statement) -> bool {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. } => {
                body.iter().any(Self::statement_references_new_target)
            }
            Statement::Var { value, .. }
            | Statement::Let { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Expression(value)
            | Statement::Throw(value)
            | Statement::Return(value)
            | Statement::Yield { value }
            | Statement::YieldDelegate { value } => Self::expression_references_new_target(value),
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                Self::expression_references_new_target(object)
                    || Self::expression_references_new_target(property)
                    || Self::expression_references_new_target(value)
            }
            Statement::Print { values } => {
                values.iter().any(Self::expression_references_new_target)
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                Self::expression_references_new_target(condition)
                    || then_branch
                        .iter()
                        .any(Self::statement_references_new_target)
                    || else_branch
                        .iter()
                        .any(Self::statement_references_new_target)
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
                .any(Self::statement_references_new_target),
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                Self::expression_references_new_target(discriminant)
                    || cases.iter().any(|case| {
                        case.test
                            .as_ref()
                            .is_some_and(Self::expression_references_new_target)
                            || case.body.iter().any(Self::statement_references_new_target)
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
                init.iter().any(Self::statement_references_new_target)
                    || condition
                        .as_ref()
                        .is_some_and(Self::expression_references_new_target)
                    || update
                        .as_ref()
                        .is_some_and(Self::expression_references_new_target)
                    || break_hook
                        .as_ref()
                        .is_some_and(Self::expression_references_new_target)
                    || body.iter().any(Self::statement_references_new_target)
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
                Self::expression_references_new_target(condition)
                    || break_hook
                        .as_ref()
                        .is_some_and(Self::expression_references_new_target)
                    || body.iter().any(Self::statement_references_new_target)
            }
            Statement::Break { .. } | Statement::Continue { .. } => false,
        }
    }

    fn expression_references_new_target(expression: &Expression) -> bool {
        match expression {
            Expression::NewTarget => true,
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                    Self::expression_references_new_target(expression)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    Self::expression_references_new_target(key)
                        || Self::expression_references_new_target(value)
                }
                ObjectEntry::Getter { key, getter } => {
                    Self::expression_references_new_target(key)
                        || Self::expression_references_new_target(getter)
                }
                ObjectEntry::Setter { key, setter } => {
                    Self::expression_references_new_target(key)
                        || Self::expression_references_new_target(setter)
                }
                ObjectEntry::Spread(expression) => {
                    Self::expression_references_new_target(expression)
                }
            }),
            Expression::Member { object, property } => {
                Self::expression_references_new_target(object)
                    || Self::expression_references_new_target(property)
            }
            Expression::SuperMember { property } => {
                Self::expression_references_new_target(property)
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => Self::expression_references_new_target(value),
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                Self::expression_references_new_target(object)
                    || Self::expression_references_new_target(property)
                    || Self::expression_references_new_target(value)
            }
            Expression::AssignSuperMember { property, value } => {
                Self::expression_references_new_target(property)
                    || Self::expression_references_new_target(value)
            }
            Expression::Binary { left, right, .. } => {
                Self::expression_references_new_target(left)
                    || Self::expression_references_new_target(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::expression_references_new_target(condition)
                    || Self::expression_references_new_target(then_expression)
                    || Self::expression_references_new_target(else_expression)
            }
            Expression::Sequence(expressions) => expressions
                .iter()
                .any(Self::expression_references_new_target),
            Expression::SuperCall { .. } => true,
            Expression::Call { callee, arguments } | Expression::New { callee, arguments } => {
                Self::expression_references_new_target(callee)
                    || arguments.iter().any(|argument| {
                        Self::expression_references_new_target(argument.expression())
                    })
            }
            Expression::Identifier(_)
            | Expression::Update { .. }
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::This
            | Expression::Sent => false,
        }
    }

    fn function_body_binds_name_to_lexical_this_function(
        functions: &[FunctionDeclaration],
        statements: &[Statement],
        binding_name: &str,
    ) -> bool {
        for statement in statements {
            match statement {
                Statement::Var { name, value }
                | Statement::Let { name, value, .. }
                | Statement::Assign { name, value }
                    if name == binding_name =>
                {
                    if let Expression::Identifier(function_name) = value
                        && functions
                            .iter()
                            .find(|function| function.name == *function_name)
                            .is_some_and(Self::function_references_lexical_this)
                    {
                        return true;
                    }
                }
                Statement::Declaration { body }
                | Statement::Block { body }
                | Statement::Labeled { body, .. }
                | Statement::With { body, .. }
                | Statement::While { body, .. }
                | Statement::DoWhile { body, .. } => {
                    if Self::function_body_binds_name_to_lexical_this_function(
                        functions,
                        body,
                        binding_name,
                    ) {
                        return true;
                    }
                }
                Statement::If {
                    then_branch,
                    else_branch,
                    ..
                } => {
                    if Self::function_body_binds_name_to_lexical_this_function(
                        functions,
                        then_branch,
                        binding_name,
                    ) || Self::function_body_binds_name_to_lexical_this_function(
                        functions,
                        else_branch,
                        binding_name,
                    ) {
                        return true;
                    }
                }
                Statement::Try {
                    body,
                    catch_setup,
                    catch_body,
                    ..
                } => {
                    if Self::function_body_binds_name_to_lexical_this_function(
                        functions,
                        body,
                        binding_name,
                    ) || Self::function_body_binds_name_to_lexical_this_function(
                        functions,
                        catch_setup,
                        binding_name,
                    ) || Self::function_body_binds_name_to_lexical_this_function(
                        functions,
                        catch_body,
                        binding_name,
                    ) {
                        return true;
                    }
                }
                Statement::Switch { cases, .. } => {
                    if cases.iter().any(|case| {
                        Self::function_body_binds_name_to_lexical_this_function(
                            functions,
                            &case.body,
                            binding_name,
                        )
                    }) {
                        return true;
                    }
                }
                Statement::For { init, body, .. } => {
                    if Self::function_body_binds_name_to_lexical_this_function(
                        functions,
                        init,
                        binding_name,
                    ) || Self::function_body_binds_name_to_lexical_this_function(
                        functions,
                        body,
                        binding_name,
                    ) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    fn referenced_enclosing_binding_captures_lexical_this(
        functions: &[FunctionDeclaration],
        function_index: usize,
        referenced: &HashSet<String>,
    ) -> bool {
        let Some(function) = functions.get(function_index) else {
            return false;
        };

        functions
            .iter()
            .enumerate()
            .any(|(candidate_index, candidate)| {
                candidate_index != function_index
                    && Self::function_references_nested_function(candidate, &function.name)
                    && referenced.iter().any(|name| {
                        Self::function_body_binds_name_to_lexical_this_function(
                            functions,
                            &candidate.body,
                            name,
                        )
                    })
            })
    }

    fn enclosing_function_name(
        functions: &[FunctionDeclaration],
        function_index: usize,
    ) -> Option<String> {
        let function = functions.get(function_index)?;
        functions
            .iter()
            .enumerate()
            .take(function_index)
            .rev()
            .find(|(_, candidate)| {
                Self::function_references_nested_function(candidate, &function.name)
            })
            .map(|(_, candidate)| candidate.name.clone())
    }

    fn home_object_name_from_define_property_target(target: &Expression) -> Option<String> {
        match target {
            Expression::Identifier(name) => Some(name.clone()),
            Expression::Member { object, property } if matches!(property.as_ref(), Expression::String(name) if name == "prototype") =>
            {
                let Expression::Identifier(name) = object.as_ref() else {
                    return None;
                };
                Some(format!("{name}.prototype"))
            }
            _ => None,
        }
    }

    fn enclosing_function_define_property_home_object_binding(
        &self,
        statements: &[Statement],
        function_name: &str,
    ) -> Option<String> {
        for statement in statements {
            match statement {
                Statement::Declaration { body }
                | Statement::Block { body }
                | Statement::Labeled { body, .. }
                | Statement::With { body, .. } => {
                    if let Some(home_object_name) = self
                        .enclosing_function_define_property_home_object_binding(body, function_name)
                    {
                        return Some(home_object_name);
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
                        CallArgument::Expression(target),
                        _,
                        CallArgument::Expression(descriptor_expression),
                        ..,
                    ] = arguments.as_slice()
                    else {
                        continue;
                    };
                    let Some(descriptor) =
                        resolve_property_descriptor_definition(descriptor_expression)
                    else {
                        continue;
                    };
                    let defines_function = descriptor
                        .value
                        .as_ref()
                        .into_iter()
                        .chain(descriptor.getter.as_ref())
                        .chain(descriptor.setter.as_ref())
                        .any(|value| {
                            matches!(value, Expression::Identifier(binding_name) if binding_name == function_name)
                                || matches!(
                                    self.infer_global_function_binding(value),
                                    Some(LocalFunctionBinding::User(ref binding_name))
                                        if binding_name == function_name
                                )
                        });
                    if defines_function
                        && let Some(home_object_name) =
                            Self::home_object_name_from_define_property_target(target)
                    {
                        return Some(home_object_name);
                    }
                }
                _ => {}
            }
        }

        None
    }

    fn register_global_bindings_in_statements(
        &mut self,
        statements: &[Statement],
        next_global_index: &mut u32,
    ) {
        for statement in statements {
            match statement {
                Statement::Declaration { body }
                | Statement::Block { body }
                | Statement::Labeled { body, .. } => {
                    self.register_global_bindings_in_statements(body, next_global_index);
                }
                Statement::Var { name, value } => {
                    self.ensure_global_binding_index(name, next_global_index);
                    if self.global_binding_kind(name).is_none() {
                        self.set_global_binding_kind(name, infer_global_expression_kind(value));
                    }
                    self.upsert_global_data_property_descriptor(
                        name,
                        self.materialize_global_expression(value),
                        Some(true),
                        true,
                        false,
                    );
                    self.update_static_global_assignment_metadata(name, value);
                }
                Statement::Let {
                    name,
                    value,
                    mutable,
                } => {
                    self.ensure_global_binding_index(name, next_global_index);
                    self.mark_global_lexical_binding(name, *mutable, next_global_index);
                    if self.global_binding_kind(name).is_none() {
                        self.set_global_binding_kind(name, infer_global_expression_kind(value));
                    }
                    self.update_static_global_assignment_metadata(name, value);
                }
                Statement::Assign { name, value } => {
                    if self.global_has_binding(name) {
                        self.update_static_global_assignment_metadata(name, value);
                    }
                }
                Statement::AssignMember {
                    object,
                    property,
                    value,
                } => {
                    self.update_global_member_assignment_metadata(object, property, value);
                }
                Statement::Expression(expression) => {
                    self.update_global_expression_metadata(expression);
                }
                _ => {}
            }
        }
    }

    fn register_hoisted_global_var_bindings_in_statements(
        &mut self,
        statements: &[Statement],
        next_global_index: &mut u32,
    ) {
        for statement in statements {
            match statement {
                Statement::Var { name, .. } => {
                    self.ensure_global_binding_index(name, next_global_index);
                    self.upsert_global_data_property_descriptor(
                        name,
                        Expression::Undefined,
                        Some(true),
                        true,
                        false,
                    );
                }
                Statement::Declaration { body }
                | Statement::Block { body }
                | Statement::Labeled { body, .. }
                | Statement::With { body, .. }
                | Statement::While { body, .. }
                | Statement::DoWhile { body, .. } => {
                    self.register_hoisted_global_var_bindings_in_statements(body, next_global_index)
                }
                Statement::If {
                    then_branch,
                    else_branch,
                    ..
                } => {
                    self.register_hoisted_global_var_bindings_in_statements(
                        then_branch,
                        next_global_index,
                    );
                    self.register_hoisted_global_var_bindings_in_statements(
                        else_branch,
                        next_global_index,
                    );
                }
                Statement::Try {
                    body,
                    catch_setup,
                    catch_body,
                    ..
                } => {
                    self.register_hoisted_global_var_bindings_in_statements(
                        body,
                        next_global_index,
                    );
                    self.register_hoisted_global_var_bindings_in_statements(
                        catch_setup,
                        next_global_index,
                    );
                    self.register_hoisted_global_var_bindings_in_statements(
                        catch_body,
                        next_global_index,
                    );
                }
                Statement::Switch { cases, .. } => {
                    for case in cases {
                        self.register_hoisted_global_var_bindings_in_statements(
                            &case.body,
                            next_global_index,
                        );
                    }
                }
                Statement::For { init, body, .. } => {
                    self.register_hoisted_global_var_bindings_in_statements(
                        init,
                        next_global_index,
                    );
                    self.register_hoisted_global_var_bindings_in_statements(
                        body,
                        next_global_index,
                    );
                }
                _ => {}
            }
        }
    }

    fn enclosing_self_binding_capture_source_name(
        functions: &[FunctionDeclaration],
        function_index: usize,
        source_name: &str,
    ) -> Option<String> {
        let function = functions.get(function_index)?;
        functions
            .iter()
            .enumerate()
            .skip(function_index + 1)
            .find(|(_, candidate)| {
                if candidate.self_binding.as_deref() != Some(source_name) {
                    return false;
                }
                if collect_referenced_binding_names_from_statements(&candidate.body)
                    .contains(&function.name)
                {
                    return true;
                }
                candidate.params.iter().any(|parameter| {
                    parameter.default.as_ref().is_some_and(|default| {
                        let mut referenced = HashSet::new();
                        collect_referenced_binding_names_from_expression(default, &mut referenced);
                        referenced.contains(&function.name)
                    })
                })
            })
            .map(|(_, candidate)| candidate.name.clone())
    }

    pub(in crate::backend::direct_wasm) fn register_global_bindings(
        &mut self,
        statements: &[Statement],
    ) {
        let mut next_global_index = self.next_allocated_global_index();
        self.register_hoisted_global_var_bindings_in_statements(statements, &mut next_global_index);
        self.register_global_bindings_in_statements(statements, &mut next_global_index);
    }

    pub(in crate::backend::direct_wasm) fn register_global_function_bindings(
        &mut self,
        functions: &[FunctionDeclaration],
    ) {
        let mut next_global_index = self.next_allocated_global_index();

        for function in functions {
            if !function.register_global {
                continue;
            }

            self.ensure_global_binding_index(&function.name, &mut next_global_index);
            self.set_global_user_function_reference(&function.name);
            self.upsert_global_data_property_descriptor(
                &function.name,
                Expression::Identifier(function.name.clone()),
                Some(true),
                true,
                false,
            );
        }
    }

    fn static_direct_eval_var_binding_source_name(
        function: &FunctionDeclaration,
        capture_name: &str,
    ) -> Option<String> {
        let capture_source_name = scoped_binding_source_name(capture_name).unwrap_or(capture_name);
        collect_static_direct_eval_var_bindings(function)
            .into_iter()
            .find_map(|name| {
                let source_name = scoped_binding_source_name(&name).unwrap_or(&name);
                (source_name == capture_source_name).then(|| source_name.to_string())
            })
    }

    fn static_direct_eval_closure_slot_name(function_name: &str, source_name: &str) -> String {
        format!("__ayy_closure_env_{function_name}_{source_name}")
    }

    fn reserve_global_function_value_capture_slots_for_assignment(
        &mut self,
        enclosing_function: &FunctionDeclaration,
        target_name: &str,
        value: &Expression,
    ) {
        if Self::function_has_local_binding_source(enclosing_function, target_name)
            || Self::function_has_parameter_binding_source(enclosing_function, target_name)
        {
            return;
        }
        if !(self.global_has_binding(target_name)
            || self.global_has_lexical_binding(target_name)
            || self.global_has_implicit_binding(target_name))
        {
            return;
        }

        let Some(LocalFunctionBinding::User(function_name)) =
            self.infer_global_function_binding(value)
        else {
            return;
        };
        let Some(capture_bindings) = self
            .state
            .function_registry
            .analysis
            .user_function_capture_bindings
            .get(&function_name)
            .filter(|captures| !captures.is_empty())
            .cloned()
        else {
            self.sync_global_function_binding(
                target_name,
                Some(LocalFunctionBinding::User(function_name)),
            );
            return;
        };

        let mut capture_slots = BTreeMap::new();
        for capture_name in capture_bindings.keys() {
            if let Some(source_name) =
                Self::static_direct_eval_var_binding_source_name(enclosing_function, capture_name)
            {
                let hidden_name = Self::static_direct_eval_closure_slot_name(
                    &enclosing_function.name,
                    &source_name,
                );
                self.ensure_implicit_global_binding(&hidden_name);
                capture_slots.insert(capture_name.clone(), hidden_name);
                continue;
            }

            let source_name = scoped_binding_source_name(capture_name).unwrap_or(capture_name);
            if Self::function_has_local_binding_source(enclosing_function, source_name)
                || Self::function_has_parameter_binding_source(enclosing_function, source_name)
            {
                let hidden_name = format!("__ayy_closure_slot_{target_name}_{capture_name}");
                self.ensure_implicit_global_binding(&hidden_name);
                capture_slots.insert(capture_name.clone(), hidden_name);
            } else if self.global_has_binding(source_name)
                || self.global_has_lexical_binding(source_name)
                || self.global_function_binding(source_name).is_some()
                || self.global_has_implicit_binding(source_name)
            {
                capture_slots.insert(capture_name.clone(), source_name.to_string());
            }
        }

        self.sync_global_function_binding(
            target_name,
            Some(LocalFunctionBinding::User(function_name)),
        );
        if !capture_slots.is_empty() {
            let key = Self::identifier_function_value_capture_slots_key(target_name);
            self.set_global_member_function_capture_slots(key, capture_slots);
        }
    }

    fn reserve_global_function_value_capture_slots_in_expression(
        &mut self,
        enclosing_function: &FunctionDeclaration,
        expression: &Expression,
    ) {
        match expression {
            Expression::Assign { name, value } => {
                self.reserve_global_function_value_capture_slots_for_assignment(
                    enclosing_function,
                    name,
                    value,
                );
                self.reserve_global_function_value_capture_slots_in_expression(
                    enclosing_function,
                    value,
                );
            }
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.reserve_global_function_value_capture_slots_in_expression(
                    enclosing_function,
                    object,
                );
                self.reserve_global_function_value_capture_slots_in_expression(
                    enclosing_function,
                    property,
                );
                self.reserve_global_function_value_capture_slots_in_expression(
                    enclosing_function,
                    value,
                );
            }
            Expression::Call { callee, arguments }
            | Expression::New { callee, arguments }
            | Expression::SuperCall { callee, arguments } => {
                self.reserve_global_function_value_capture_slots_in_expression(
                    enclosing_function,
                    callee,
                );
                for argument in arguments {
                    match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.reserve_global_function_value_capture_slots_in_expression(
                                enclosing_function,
                                expression,
                            );
                        }
                    }
                }
            }
            Expression::Array(elements) => {
                for element in elements {
                    match element {
                        ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                            self.reserve_global_function_value_capture_slots_in_expression(
                                enclosing_function,
                                expression,
                            );
                        }
                    }
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            self.reserve_global_function_value_capture_slots_in_expression(
                                enclosing_function,
                                key,
                            );
                            self.reserve_global_function_value_capture_slots_in_expression(
                                enclosing_function,
                                value,
                            );
                        }
                        ObjectEntry::Getter { key, getter } => {
                            self.reserve_global_function_value_capture_slots_in_expression(
                                enclosing_function,
                                key,
                            );
                            self.reserve_global_function_value_capture_slots_in_expression(
                                enclosing_function,
                                getter,
                            );
                        }
                        ObjectEntry::Setter { key, setter } => {
                            self.reserve_global_function_value_capture_slots_in_expression(
                                enclosing_function,
                                key,
                            );
                            self.reserve_global_function_value_capture_slots_in_expression(
                                enclosing_function,
                                setter,
                            );
                        }
                        ObjectEntry::Spread(expression) => {
                            self.reserve_global_function_value_capture_slots_in_expression(
                                enclosing_function,
                                expression,
                            );
                        }
                    }
                }
            }
            Expression::Member { object, property } => {
                self.reserve_global_function_value_capture_slots_in_expression(
                    enclosing_function,
                    object,
                );
                self.reserve_global_function_value_capture_slots_in_expression(
                    enclosing_function,
                    property,
                );
            }
            Expression::SuperMember { property } => {
                self.reserve_global_function_value_capture_slots_in_expression(
                    enclosing_function,
                    property,
                );
            }
            Expression::AssignSuperMember { property, value } => {
                self.reserve_global_function_value_capture_slots_in_expression(
                    enclosing_function,
                    property,
                );
                self.reserve_global_function_value_capture_slots_in_expression(
                    enclosing_function,
                    value,
                );
            }
            Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => {
                self.reserve_global_function_value_capture_slots_in_expression(
                    enclosing_function,
                    value,
                );
            }
            Expression::Binary { left, right, .. } => {
                self.reserve_global_function_value_capture_slots_in_expression(
                    enclosing_function,
                    left,
                );
                self.reserve_global_function_value_capture_slots_in_expression(
                    enclosing_function,
                    right,
                );
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.reserve_global_function_value_capture_slots_in_expression(
                    enclosing_function,
                    condition,
                );
                self.reserve_global_function_value_capture_slots_in_expression(
                    enclosing_function,
                    then_expression,
                );
                self.reserve_global_function_value_capture_slots_in_expression(
                    enclosing_function,
                    else_expression,
                );
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    self.reserve_global_function_value_capture_slots_in_expression(
                        enclosing_function,
                        expression,
                    );
                }
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
            | Expression::Update { .. } => {}
        }
    }

    fn reserve_global_function_value_capture_slots_in_statements(
        &mut self,
        enclosing_function: &FunctionDeclaration,
        statements: &[Statement],
    ) {
        for statement in statements {
            match statement {
                Statement::Declaration { body }
                | Statement::Block { body }
                | Statement::Labeled { body, .. }
                | Statement::With { body, .. }
                | Statement::While { body, .. }
                | Statement::DoWhile { body, .. } => {
                    self.reserve_global_function_value_capture_slots_in_statements(
                        enclosing_function,
                        body,
                    );
                }
                Statement::If {
                    condition,
                    then_branch,
                    else_branch,
                } => {
                    self.reserve_global_function_value_capture_slots_in_expression(
                        enclosing_function,
                        condition,
                    );
                    self.reserve_global_function_value_capture_slots_in_statements(
                        enclosing_function,
                        then_branch,
                    );
                    self.reserve_global_function_value_capture_slots_in_statements(
                        enclosing_function,
                        else_branch,
                    );
                }
                Statement::Try {
                    body,
                    catch_setup,
                    catch_body,
                    ..
                } => {
                    self.reserve_global_function_value_capture_slots_in_statements(
                        enclosing_function,
                        body,
                    );
                    self.reserve_global_function_value_capture_slots_in_statements(
                        enclosing_function,
                        catch_setup,
                    );
                    self.reserve_global_function_value_capture_slots_in_statements(
                        enclosing_function,
                        catch_body,
                    );
                }
                Statement::Switch {
                    discriminant,
                    cases,
                    ..
                } => {
                    self.reserve_global_function_value_capture_slots_in_expression(
                        enclosing_function,
                        discriminant,
                    );
                    for case in cases {
                        if let Some(test) = &case.test {
                            self.reserve_global_function_value_capture_slots_in_expression(
                                enclosing_function,
                                test,
                            );
                        }
                        self.reserve_global_function_value_capture_slots_in_statements(
                            enclosing_function,
                            &case.body,
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
                    self.reserve_global_function_value_capture_slots_in_statements(
                        enclosing_function,
                        init,
                    );
                    if let Some(condition) = condition {
                        self.reserve_global_function_value_capture_slots_in_expression(
                            enclosing_function,
                            condition,
                        );
                    }
                    if let Some(update) = update {
                        self.reserve_global_function_value_capture_slots_in_expression(
                            enclosing_function,
                            update,
                        );
                    }
                    if let Some(break_hook) = break_hook {
                        self.reserve_global_function_value_capture_slots_in_expression(
                            enclosing_function,
                            break_hook,
                        );
                    }
                    self.reserve_global_function_value_capture_slots_in_statements(
                        enclosing_function,
                        body,
                    );
                }
                Statement::Var { name, value }
                | Statement::Let { name, value, .. }
                | Statement::Assign { name, value } => {
                    self.reserve_global_function_value_capture_slots_for_assignment(
                        enclosing_function,
                        name,
                        value,
                    );
                    self.reserve_global_function_value_capture_slots_in_expression(
                        enclosing_function,
                        value,
                    );
                }
                Statement::AssignMember {
                    object,
                    property,
                    value,
                } => {
                    self.reserve_global_function_value_capture_slots_in_expression(
                        enclosing_function,
                        object,
                    );
                    self.reserve_global_function_value_capture_slots_in_expression(
                        enclosing_function,
                        property,
                    );
                    self.reserve_global_function_value_capture_slots_in_expression(
                        enclosing_function,
                        value,
                    );
                }
                Statement::Expression(value)
                | Statement::Return(value)
                | Statement::Throw(value)
                | Statement::Yield { value }
                | Statement::YieldDelegate { value } => {
                    self.reserve_global_function_value_capture_slots_in_expression(
                        enclosing_function,
                        value,
                    );
                }
                Statement::Print { values } => {
                    for value in values {
                        self.reserve_global_function_value_capture_slots_in_expression(
                            enclosing_function,
                            value,
                        );
                    }
                }
                Statement::Break { .. } | Statement::Continue { .. } => {}
            }
        }
    }

    pub(in crate::backend::direct_wasm) fn reserve_global_function_value_capture_slots(
        &mut self,
        functions: &[FunctionDeclaration],
    ) {
        for function in functions {
            for parameter in &function.params {
                if let Some(default) = &parameter.default {
                    self.reserve_global_function_value_capture_slots_in_expression(
                        function, default,
                    );
                }
            }
            self.reserve_global_function_value_capture_slots_in_statements(
                function,
                &function.body,
            );
        }
    }

    fn capture_scan_expression_mentions_direct_eval(expression: &Expression) -> bool {
        match expression {
            Expression::Call { callee, arguments } => {
                matches!(callee.as_ref(), Expression::Identifier(name) if name == "eval")
                    || Self::capture_scan_expression_mentions_direct_eval(callee)
                    || arguments.iter().any(|argument| match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            Self::capture_scan_expression_mentions_direct_eval(expression)
                        }
                    })
            }
            Expression::Member { object, property } => {
                Self::capture_scan_expression_mentions_direct_eval(object)
                    || Self::capture_scan_expression_mentions_direct_eval(property)
            }
            Expression::Assign { value, .. } => {
                Self::capture_scan_expression_mentions_direct_eval(value)
            }
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                Self::capture_scan_expression_mentions_direct_eval(object)
                    || Self::capture_scan_expression_mentions_direct_eval(property)
                    || Self::capture_scan_expression_mentions_direct_eval(value)
            }
            Expression::AssignSuperMember { property, value } => {
                Self::capture_scan_expression_mentions_direct_eval(property)
                    || Self::capture_scan_expression_mentions_direct_eval(value)
            }
            Expression::Await(expression)
            | Expression::EnumerateKeys(expression)
            | Expression::GetIterator(expression)
            | Expression::IteratorClose(expression)
            | Expression::Unary { expression, .. } => {
                Self::capture_scan_expression_mentions_direct_eval(expression)
            }
            Expression::Binary { left, right, .. } => {
                Self::capture_scan_expression_mentions_direct_eval(left)
                    || Self::capture_scan_expression_mentions_direct_eval(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::capture_scan_expression_mentions_direct_eval(condition)
                    || Self::capture_scan_expression_mentions_direct_eval(then_expression)
                    || Self::capture_scan_expression_mentions_direct_eval(else_expression)
            }
            Expression::Sequence(expressions) => expressions
                .iter()
                .any(Self::capture_scan_expression_mentions_direct_eval),
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                    Self::capture_scan_expression_mentions_direct_eval(expression)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    Self::capture_scan_expression_mentions_direct_eval(key)
                        || Self::capture_scan_expression_mentions_direct_eval(value)
                }
                ObjectEntry::Getter { key, getter } => {
                    Self::capture_scan_expression_mentions_direct_eval(key)
                        || Self::capture_scan_expression_mentions_direct_eval(getter)
                }
                ObjectEntry::Setter { key, setter } => {
                    Self::capture_scan_expression_mentions_direct_eval(key)
                        || Self::capture_scan_expression_mentions_direct_eval(setter)
                }
                ObjectEntry::Spread(expression) => {
                    Self::capture_scan_expression_mentions_direct_eval(expression)
                }
            }),
            Expression::SuperCall { callee, arguments } | Expression::New { callee, arguments } => {
                Self::capture_scan_expression_mentions_direct_eval(callee)
                    || arguments.iter().any(|argument| match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            Self::capture_scan_expression_mentions_direct_eval(expression)
                        }
                    })
            }
            Expression::Identifier(_)
            | Expression::This
            | Expression::SuperMember { .. }
            | Expression::Update { .. }
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::NewTarget
            | Expression::Sent => false,
        }
    }

    fn capture_scan_statement_mentions_direct_eval(statement: &Statement) -> bool {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. } => body
                .iter()
                .any(Self::capture_scan_statement_mentions_direct_eval),
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                Self::capture_scan_expression_mentions_direct_eval(condition)
                    || then_branch
                        .iter()
                        .any(Self::capture_scan_statement_mentions_direct_eval)
                    || else_branch
                        .iter()
                        .any(Self::capture_scan_statement_mentions_direct_eval)
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                Self::capture_scan_expression_mentions_direct_eval(discriminant)
                    || cases.iter().any(|case| {
                        case.test
                            .as_ref()
                            .is_some_and(Self::capture_scan_expression_mentions_direct_eval)
                            || case
                                .body
                                .iter()
                                .any(Self::capture_scan_statement_mentions_direct_eval)
                    })
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                body.iter()
                    .any(Self::capture_scan_statement_mentions_direct_eval)
                    || catch_setup
                        .iter()
                        .any(Self::capture_scan_statement_mentions_direct_eval)
                    || catch_body
                        .iter()
                        .any(Self::capture_scan_statement_mentions_direct_eval)
            }
            Statement::While {
                condition, body, ..
            }
            | Statement::DoWhile {
                condition, body, ..
            } => {
                Self::capture_scan_expression_mentions_direct_eval(condition)
                    || body
                        .iter()
                        .any(Self::capture_scan_statement_mentions_direct_eval)
            }
            Statement::For {
                init,
                condition,
                update,
                body,
                ..
            } => {
                init.iter()
                    .any(Self::capture_scan_statement_mentions_direct_eval)
                    || condition
                        .as_ref()
                        .is_some_and(Self::capture_scan_expression_mentions_direct_eval)
                    || update
                        .as_ref()
                        .is_some_and(Self::capture_scan_expression_mentions_direct_eval)
                    || body
                        .iter()
                        .any(Self::capture_scan_statement_mentions_direct_eval)
            }
            Statement::Var { value, .. }
            | Statement::Let { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Return(value)
            | Statement::Throw(value)
            | Statement::Expression(value)
            | Statement::Yield { value }
            | Statement::YieldDelegate { value } => {
                Self::capture_scan_expression_mentions_direct_eval(value)
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                Self::capture_scan_expression_mentions_direct_eval(object)
                    || Self::capture_scan_expression_mentions_direct_eval(property)
                    || Self::capture_scan_expression_mentions_direct_eval(value)
            }
            Statement::Print { values } => values
                .iter()
                .any(Self::capture_scan_expression_mentions_direct_eval),
            Statement::Break { .. } | Statement::Continue { .. } => false,
        }
    }

    fn capture_scan_function_mentions_direct_eval(function: &FunctionDeclaration) -> bool {
        function
            .body
            .iter()
            .any(Self::capture_scan_statement_mentions_direct_eval)
    }

    pub(in crate::backend::direct_wasm) fn register_user_function_capture_bindings(
        &mut self,
        functions: &[FunctionDeclaration],
    ) {
        self.clear_user_function_capture_bindings();
        let trace_capture_bindings = std::env::var_os("AYY_TRACE_CAPTURE_BINDINGS").is_some();

        for (function_index, function) in functions.iter().enumerate() {
            if function.lexical_this
                && let Some(parent_name) = Self::enclosing_function_name(functions, function_index)
                && let Some(home_object_name) = self
                    .user_function_home_object_binding(&parent_name)
                    .or_else(|| self.find_global_home_object_binding_name(&parent_name))
            {
                self.set_user_function_home_object_binding(&function.name, &home_object_name);
            }
            if self
                .user_function_home_object_binding(&function.name)
                .is_none()
                && let Some(parent_name) = Self::enclosing_function_name(functions, function_index)
                && let Some(parent_function) = functions
                    .iter()
                    .find(|candidate| candidate.name == parent_name)
                && let Some(home_object_name) = self
                    .enclosing_function_define_property_home_object_binding(
                        &parent_function.body,
                        &function.name,
                    )
            {
                self.set_user_function_home_object_binding(&function.name, &home_object_name);
            }
            let scope_bindings = collect_function_constructor_local_bindings(function)
                .into_iter()
                .map(|name| {
                    scoped_binding_source_name(&name)
                        .unwrap_or(&name)
                        .to_string()
                })
                .collect::<HashSet<_>>();
            let mut referenced = collect_referenced_binding_names_from_statements(&function.body);
            let mut references_lexical_this =
                function.lexical_this && statements_reference_this(&function.body);
            let mut references_lexical_new_target =
                Self::function_references_lexical_new_target(function);
            for parameter in &function.params {
                if let Some(default) = &parameter.default {
                    collect_referenced_binding_names_from_expression(default, &mut referenced);
                    if function.lexical_this && expression_references_this(default) {
                        references_lexical_this = true;
                    }
                    if function.lexical_this && Self::expression_references_new_target(default) {
                        references_lexical_new_target = true;
                    }
                }
            }
            if !references_lexical_this
                && Self::referenced_enclosing_binding_captures_lexical_this(
                    functions,
                    function_index,
                    &referenced,
                )
            {
                references_lexical_this = true;
            }
            referenced.extend(function.synthetic_capture_bindings.iter().cloned());
            if references_lexical_this {
                referenced.insert("this".to_string());
            }
            if references_lexical_new_target {
                referenced.insert("new.target".to_string());
            }
            let function_mentions_direct_eval =
                Self::capture_scan_function_mentions_direct_eval(function);
            let eval_local_function_bindings = self
                .state
                .function_registry
                .analysis
                .eval_local_function_bindings
                .get(&function.name)
                .map(|bindings| bindings.keys().cloned().collect::<HashSet<_>>())
                .unwrap_or_default();
            let mut captures = HashMap::new();

            for name in referenced.iter().cloned() {
                let is_scoped_binding = scoped_binding_source_name(&name).is_some();
                let source_name = scoped_binding_source_name(&name)
                    .unwrap_or(&name)
                    .to_string();
                let is_eval_local_function_binding =
                    eval_local_function_bindings.iter().any(|binding_name| {
                        binding_name == &source_name
                            || scoped_binding_source_name(binding_name)
                                .is_some_and(|binding_source| binding_source == source_name)
                    });
                let is_synthetic_capture = function
                    .synthetic_capture_bindings
                    .iter()
                    .any(|binding| binding == &name || binding == &source_name);
                if !is_synthetic_capture && is_eval_local_function_binding {
                    continue;
                }
                let capture_source_name = Self::enclosing_self_binding_capture_source_name(
                    functions,
                    function_index,
                    &source_name,
                )
                .unwrap_or_else(|| source_name.clone());
                let captures_enclosing_body_local =
                    Self::nested_function_captures_enclosing_body_local(
                        functions,
                        function_index,
                        &source_name,
                    );
                let captures_enclosing_parameter_local =
                    Self::nested_function_captures_enclosing_parameter_local(
                        functions,
                        function_index,
                        &source_name,
                    );
                let is_own_parameter_binding =
                    Self::function_has_parameter_binding_source(function, &source_name);
                if !is_synthetic_capture && is_own_parameter_binding {
                    continue;
                }
                if !is_synthetic_capture
                    && function_mentions_direct_eval
                    && !captures_enclosing_body_local
                    && !captures_enclosing_parameter_local
                {
                    continue;
                }
                if !is_synthetic_capture
                    && !captures_enclosing_body_local
                    && !captures_enclosing_parameter_local
                    && (scope_bindings.contains(&source_name)
                        || (!is_scoped_binding && self.contains_user_function(&source_name))
                        || self.global_has_binding(&source_name)
                        || self.global_has_lexical_binding(&source_name)
                        || self.global_function_binding(&source_name).is_some()
                        || self.global_has_implicit_binding(&source_name)
                        || is_builtin_like_capture_identifier(&source_name))
                {
                    continue;
                }

                let hidden_name = format!(
                    "__ayy_capture_binding__{}__{}",
                    function.name, capture_source_name
                );
                self.ensure_implicit_global_binding(&hidden_name);
                captures.entry(capture_source_name).or_insert(hidden_name);
            }

            if !captures.is_empty() {
                if trace_capture_bindings {
                    eprintln!(
                        "capture_bindings function_index={function_index} function={} captures={captures:?}",
                        function.name
                    );
                }
                self.set_user_function_capture_bindings(&function.name, captures);
            } else if trace_capture_bindings {
                eprintln!(
                    "capture_bindings function_index={function_index} function={} captures={{}} referenced={referenced:?} scope={scope_bindings:?}",
                    function.name
                );
            }
        }
        self.reserve_class_field_lexical_this_member_capture_slots();
    }

    fn reserve_class_field_lexical_this_member_capture_slots(&mut self) {
        for (key, binding) in self.global_member_function_binding_entries() {
            let LocalFunctionBinding::User(function_name) = binding else {
                continue;
            };
            let Some(function) = self.registered_function(&function_name) else {
                continue;
            };
            if !function.lexical_this || !function.direct_eval_in_class_field_initializer {
                continue;
            }
            if !self
                .state
                .function_registry
                .analysis
                .user_function_capture_bindings
                .get(&function_name)
                .is_some_and(|captures| captures.contains_key("this"))
            {
                continue;
            }
            let Some(home_object_name) = self
                .user_function(&function_name)
                .and_then(|function| function.home_object_binding.clone())
            else {
                continue;
            };
            if home_object_name.ends_with(".prototype") {
                continue;
            }
            let mut capture_slots = self
                .global_member_function_capture_slots(&key)
                .cloned()
                .unwrap_or_default();
            capture_slots
                .entry("this".to_string())
                .or_insert(home_object_name);
            self.set_global_member_function_capture_slots(key, capture_slots);
        }
    }

    fn reserve_returned_member_capture_slots_for_global_assignment(
        &mut self,
        name: &str,
        value: &Expression,
    ) {
        if let Expression::Call { callee, .. } = value
            && let Some(LocalFunctionBinding::User(function_name)) =
                self.infer_global_function_binding(callee)
            && let Some(Expression::Identifier(returned_name)) =
                self.direct_user_function_return_expression(&function_name, 0)
            && self.contains_user_function(&returned_name)
        {
            self.sync_global_function_binding(
                name,
                Some(LocalFunctionBinding::User(returned_name)),
            );
        }
        let inherited_member_bindings = self.global_inherited_member_function_bindings(value);
        if inherited_member_bindings.is_empty() {
            return;
        }

        for binding in inherited_member_bindings {
            let LocalFunctionBinding::User(member_function_name) = &binding.binding else {
                continue;
            };
            let Some(capture_bindings) = self
                .state
                .function_registry
                .analysis
                .user_function_capture_bindings
                .get(member_function_name)
                .filter(|captures| !captures.is_empty())
                .cloned()
            else {
                continue;
            };

            let inherited_capture_slots =
                self.prototype_capture_slots_for_new_assignment(value, &binding.property);
            let key = MemberFunctionBindingKey {
                target: match binding.target {
                    ReturnedMemberFunctionBindingTarget::Value => {
                        MemberFunctionBindingTarget::Identifier(name.to_string())
                    }
                    ReturnedMemberFunctionBindingTarget::Prototype => {
                        MemberFunctionBindingTarget::Prototype(name.to_string())
                    }
                },
                property: MemberFunctionBindingProperty::String(binding.property.clone()),
            };
            let mut capture_slots = self
                .global_member_function_capture_slots(&key)
                .cloned()
                .or(inherited_capture_slots)
                .unwrap_or_default();
            self.set_global_member_function_binding(key.clone(), binding.binding.clone());

            for capture_name in capture_bindings.keys() {
                if capture_slots.contains_key(capture_name) {
                    continue;
                }
                let hidden_name = format!("__ayy_closure_slot_{name}_{capture_name}");
                self.ensure_implicit_global_binding(&hidden_name);
                capture_slots.insert(capture_name.clone(), hidden_name);
            }

            if !capture_slots.is_empty() {
                self.set_global_member_function_capture_slots(key, capture_slots);
            }
        }
    }

    fn prototype_capture_slots_for_new_assignment(
        &self,
        value: &Expression,
        property: &str,
    ) -> Option<BTreeMap<String, String>> {
        let Expression::New { callee, .. } = value else {
            return None;
        };
        let Expression::Identifier(constructor_name) = callee.as_ref() else {
            return None;
        };
        let mut constructor_names = vec![constructor_name.clone()];
        if let Some(LocalFunctionBinding::User(function_name)) =
            self.infer_global_function_binding(callee)
            && let Some(function) = self.registered_function(&function_name)
        {
            if let Some(self_binding) = function.self_binding.as_ref() {
                constructor_names.push(self_binding.clone());
            }
            if let Some(top_level_binding) = function.top_level_binding.as_ref() {
                constructor_names.push(top_level_binding.clone());
            }
        }
        let property = MemberFunctionBindingProperty::String(property.to_string());
        constructor_names.into_iter().find_map(|constructor_name| {
            let key = MemberFunctionBindingKey {
                target: MemberFunctionBindingTarget::Prototype(constructor_name),
                property: property.clone(),
            };
            self.global_member_function_capture_slots(&key).cloned()
        })
    }

    fn reserve_global_returned_member_capture_slots_in_statements(
        &mut self,
        statements: &[Statement],
    ) {
        for statement in statements {
            match statement {
                Statement::Declaration { body }
                | Statement::Block { body }
                | Statement::Labeled { body, .. } => {
                    self.reserve_global_returned_member_capture_slots_in_statements(body);
                }
                Statement::Var { name, value }
                | Statement::Let { name, value, .. }
                | Statement::Assign { name, value }
                    if self.global_has_binding(name)
                        || self.global_has_lexical_binding(name)
                        || self.global_has_implicit_binding(name) =>
                {
                    self.reserve_returned_member_capture_slots_for_global_assignment(name, value);
                }
                _ => {}
            }
        }
    }

    pub(in crate::backend::direct_wasm) fn reserve_global_returned_member_capture_slots(
        &mut self,
        statements: &[Statement],
    ) {
        self.reserve_global_returned_member_capture_slots_in_statements(statements);
    }

    pub(in crate::backend::direct_wasm) fn reserve_global_array_runtime_state_bindings(
        &mut self,
        program: &Program,
    ) {
        let global_array_names = self
            .global_array_binding_entries()
            .into_iter()
            .map(|(name, _)| name)
            .collect::<HashSet<_>>();
        for function in &program.functions {
            let local_bindings = collect_function_constructor_local_bindings(function)
                .into_iter()
                .map(|name| {
                    scoped_binding_source_name(&name)
                        .unwrap_or(&name)
                        .to_string()
                })
                .collect::<HashSet<_>>();
            let mut referenced = collect_referenced_binding_names_from_statements(&function.body);
            for parameter in &function.params {
                if let Some(default) = &parameter.default {
                    collect_referenced_binding_names_from_expression(default, &mut referenced);
                }
            }
            for name in referenced {
                let source_name = scoped_binding_source_name(&name).unwrap_or(&name);
                if local_bindings.contains(source_name) {
                    continue;
                }
                if global_array_names.contains(source_name) {
                    self.mark_global_array_with_runtime_state(source_name);
                }
            }
        }
    }

    pub(in crate::backend::direct_wasm) fn reserve_function_constructor_implicit_global_bindings(
        &mut self,
        program: &Program,
    ) -> DirectResult<()> {
        let mut names = BTreeSet::new();
        for function in &program.functions {
            let scope = collect_function_constructor_local_bindings(function);
            collect_implicit_globals_from_statements(
                &function.body,
                function.strict,
                &scope,
                &mut names,
            )?;
        }

        for name in names {
            if self.global_has_binding(&name) || self.global_has_implicit_binding(&name) {
                continue;
            }
            self.create_implicit_global_binding(&name);
        }

        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn ensure_implicit_global_binding(
        &mut self,
        name: &str,
    ) -> ImplicitGlobalBinding {
        self.create_implicit_global_binding(name)
    }

    pub(in crate::backend::direct_wasm) fn next_allocated_global_index(&self) -> u32 {
        self.next_available_global_index()
    }

    pub(in crate::backend::direct_wasm) fn reserve_global_runtime_prototype_binding_globals(
        &mut self,
    ) {
        let mut names = self.runtime_prototype_binding_names();
        names.sort();
        let mut next_global_index = self.next_allocated_global_index();
        for name in names {
            self.set_runtime_prototype_binding_global_index(&name, next_global_index);
            next_global_index += 1;
        }
    }
}
