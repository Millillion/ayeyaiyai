use super::*;

impl<'a> FunctionCompiler<'a> {
    fn seed_lexical_binding_state(
        declaration: Option<&FunctionDeclaration>,
        next_local_index: &mut u32,
    ) -> (HashMap<String, u32>, HashSet<String>) {
        fn collect_lexical_bindings(
            statements: &[Statement],
            initialized_locals: &mut HashMap<String, u32>,
            immutable_bindings: &mut HashSet<String>,
            next_local_index: &mut u32,
        ) {
            for statement in statements {
                match statement {
                    Statement::Declaration { body }
                    | Statement::Block { body }
                    | Statement::Labeled { body, .. }
                    | Statement::With { body, .. } => collect_lexical_bindings(
                        body,
                        initialized_locals,
                        immutable_bindings,
                        next_local_index,
                    ),
                    Statement::If {
                        then_branch,
                        else_branch,
                        ..
                    } => {
                        collect_lexical_bindings(
                            then_branch,
                            initialized_locals,
                            immutable_bindings,
                            next_local_index,
                        );
                        collect_lexical_bindings(
                            else_branch,
                            initialized_locals,
                            immutable_bindings,
                            next_local_index,
                        );
                    }
                    Statement::Try {
                        body,
                        catch_setup,
                        catch_body,
                        ..
                    } => {
                        collect_lexical_bindings(
                            body,
                            initialized_locals,
                            immutable_bindings,
                            next_local_index,
                        );
                        collect_lexical_bindings(
                            catch_setup,
                            initialized_locals,
                            immutable_bindings,
                            next_local_index,
                        );
                        collect_lexical_bindings(
                            catch_body,
                            initialized_locals,
                            immutable_bindings,
                            next_local_index,
                        );
                    }
                    Statement::Switch { cases, .. } => {
                        for case in cases {
                            collect_lexical_bindings(
                                &case.body,
                                initialized_locals,
                                immutable_bindings,
                                next_local_index,
                            );
                        }
                    }
                    Statement::For { init, body, .. } => {
                        collect_lexical_bindings(
                            init,
                            initialized_locals,
                            immutable_bindings,
                            next_local_index,
                        );
                        collect_lexical_bindings(
                            body,
                            initialized_locals,
                            immutable_bindings,
                            next_local_index,
                        );
                    }
                    Statement::While { body, .. } | Statement::DoWhile { body, .. } => {
                        collect_lexical_bindings(
                            body,
                            initialized_locals,
                            immutable_bindings,
                            next_local_index,
                        );
                    }
                    Statement::Let { name, mutable, .. } => {
                        if !initialized_locals.contains_key(name) {
                            initialized_locals.insert(name.clone(), *next_local_index);
                            *next_local_index += 1;
                        }
                        if !*mutable {
                            immutable_bindings.insert(name.clone());
                        }
                    }
                    _ => {}
                }
            }
        }

        let mut initialized_locals = HashMap::new();
        let mut immutable_bindings = HashSet::new();
        if let Some(declaration) = declaration {
            collect_lexical_bindings(
                &declaration.body,
                &mut initialized_locals,
                &mut immutable_bindings,
                next_local_index,
            );
        }
        (initialized_locals, immutable_bindings)
    }

    pub(super) fn prepare_binding_state(
        module: &DirectWasmCompiler,
        user_function: Option<&UserFunction>,
        declaration: Option<&FunctionDeclaration>,
        total_param_count: u32,
        next_local_index: &mut u32,
        global_binding_environment: &GlobalBindingEnvironment,
        parameter_names: &[String],
        parameter_bindings: &HashMap<String, Option<LocalFunctionBinding>>,
        parameter_value_bindings: &HashMap<String, Option<Expression>>,
        parameter_array_bindings: &HashMap<String, Option<ArrayValueBinding>>,
        parameter_object_bindings: &HashMap<String, Option<ObjectValueBinding>>,
    ) -> EntryBindingState {
        let mut bindings = Self::seed_parameter_binding_state(
            module,
            global_binding_environment,
            declaration,
            parameter_names,
            parameter_bindings,
            parameter_value_bindings,
            parameter_array_bindings,
            parameter_object_bindings,
        );
        Self::add_fallback_local(&mut bindings, total_param_count);
        Self::allocate_function_scope_locals(&mut bindings, user_function, next_local_index);
        let (local_lexical_initialized_locals, immutable_local_bindings) =
            Self::seed_lexical_binding_state(declaration, next_local_index);
        bindings.static_bindings.local_lexical_initialized_locals =
            local_lexical_initialized_locals;
        bindings.static_bindings.immutable_local_bindings = immutable_local_bindings;
        Self::apply_special_function_bindings(
            &mut bindings,
            user_function,
            declaration,
            global_binding_environment,
        );
        bindings
    }

    fn seed_parameter_binding_state(
        module: &DirectWasmCompiler,
        global_binding_environment: &GlobalBindingEnvironment,
        declaration: Option<&FunctionDeclaration>,
        parameter_names: &[String],
        parameter_bindings: &HashMap<String, Option<LocalFunctionBinding>>,
        parameter_value_bindings: &HashMap<String, Option<Expression>>,
        parameter_array_bindings: &HashMap<String, Option<ArrayValueBinding>>,
        parameter_object_bindings: &HashMap<String, Option<ObjectValueBinding>>,
    ) -> EntryBindingState {
        let mut locals = HashMap::new();
        let mut local_kinds = HashMap::new();
        let mut local_function_bindings = HashMap::new();
        for (index, param) in parameter_names.iter().enumerate() {
            if !locals.contains_key(param) {
                locals.insert(param.clone(), index as u32);
            }
            local_kinds.insert(param.clone(), StaticValueKind::Unknown);
            if let Some(Some(binding)) = parameter_bindings.get(param) {
                local_function_bindings.insert(param.clone(), binding.clone());
            }
        }

        let rest_parameter_names = declaration
            .map(|declaration| {
                declaration
                    .params
                    .iter()
                    .filter(|parameter| parameter.rest)
                    .map(|parameter| parameter.name.clone())
                    .collect::<HashSet<_>>()
            })
            .unwrap_or_default();

        let mut local_value_bindings = HashMap::new();
        for param in parameter_names {
            if let Some(Some(binding)) = parameter_value_bindings.get(param) {
                local_value_bindings.insert(param.clone(), binding.clone());
            }
        }

        let mut local_array_bindings = HashMap::new();
        for param in parameter_names {
            if let Some(Some(binding)) = parameter_array_bindings.get(param) {
                local_array_bindings.insert(param.clone(), binding.clone());
            } else if rest_parameter_names.contains(param) {
                local_array_bindings
                    .insert(param.clone(), ArrayValueBinding { values: Vec::new() });
                local_kinds.insert(param.clone(), StaticValueKind::Object);
            }
        }

        let mut local_object_bindings = HashMap::new();
        for param in parameter_names {
            if let Some(Some(binding)) = parameter_object_bindings.get(param) {
                local_object_bindings.insert(param.clone(), binding.clone());
                local_kinds.insert(param.clone(), StaticValueKind::Object);
                continue;
            }
            if let Some(Some(binding)) = parameter_value_bindings.get(param) {
                let resolved_binding = module
                    .materialize_global_expression_with_state(
                        binding,
                        &HashMap::new(),
                        &global_binding_environment.value_bindings,
                        &global_binding_environment.object_bindings,
                    )
                    .unwrap_or_else(|| module.materialize_global_expression(binding));
                if let Some(object_binding) = module.infer_global_object_binding(&resolved_binding)
                {
                    local_object_bindings.insert(param.clone(), object_binding);
                    local_kinds.insert(param.clone(), StaticValueKind::Object);
                }
            }
        }

        EntryBindingState {
            locals,
            static_bindings: PreparedLocalStaticBindings {
                local_kinds,
                local_value_bindings,
                local_function_bindings,
                local_array_bindings,
                local_object_bindings,
                local_lexical_initialized_locals: HashMap::new(),
                immutable_local_bindings: HashSet::new(),
            },
        }
    }

    fn add_fallback_local(bindings: &mut EntryBindingState, total_param_count: u32) {
        let fallback_local_name = "__ayy_fallback_local";
        bindings
            .locals
            .insert(fallback_local_name.to_string(), total_param_count);
        bindings
            .static_bindings
            .local_kinds
            .insert(fallback_local_name.to_string(), StaticValueKind::Unknown);
    }

    fn allocate_function_scope_locals(
        bindings: &mut EntryBindingState,
        user_function: Option<&UserFunction>,
        next_local_index: &mut u32,
    ) {
        if let Some(user_function) = user_function {
            let mut scope_bindings = user_function
                .scope_bindings
                .iter()
                .cloned()
                .collect::<Vec<_>>();
            scope_bindings.sort();
            for binding in scope_bindings {
                if binding == "arguments" || bindings.locals.contains_key(&binding) {
                    continue;
                }
                bindings.locals.insert(binding.clone(), *next_local_index);
                bindings
                    .static_bindings
                    .local_kinds
                    .insert(binding, StaticValueKind::Unknown);
                *next_local_index += 1;
            }
        }
    }

    pub(super) fn recover_parameter_object_bindings_from_value_bindings(&mut self) {
        let parameter_names = self.state.parameters.parameter_names.clone();
        for parameter_name in parameter_names {
            if self
                .state
                .speculation
                .static_semantics
                .local_object_binding(&parameter_name)
                .is_some()
            {
                continue;
            }
            let Some(value_binding) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(&parameter_name)
                .cloned()
            else {
                continue;
            };
            self.update_local_object_binding(&parameter_name, &value_binding);
        }
    }
}
