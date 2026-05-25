use super::*;

const MODULE_NAMESPACE_DESCRIPTOR_MODULE_INDEX: &str = "__ayy$module$namespace$moduleIndex";
const MODULE_REEXPORT_DESCRIPTOR_MODULE_INDEX: &str = "__ayy$module$reexport$moduleIndex";
const MODULE_REEXPORT_DESCRIPTOR_NAME: &str = "__ayy$module$reexport$name";

impl DirectWasmCompiler {
    pub(in crate::backend::direct_wasm) fn dynamic_import_then_callback_namespace_argument(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
        aliases: &HashMap<String, Option<LocalFunctionBinding>>,
    ) -> Option<(String, Expression)> {
        let import_arguments = self.dynamic_import_then_source_arguments(callee)?;
        let callback = match arguments.first()? {
            CallArgument::Expression(callback) => callback,
            CallArgument::Spread(_) => return None,
        };
        let LocalFunctionBinding::User(callback_name) =
            self.resolve_function_binding_from_expression_with_aliases(callback, aliases)?
        else {
            return None;
        };
        let namespace_binding =
            self.static_dynamic_import_module_namespace_object_binding(import_arguments)?;
        Some((
            callback_name,
            object_binding_to_expression(&namespace_binding),
        ))
    }

    fn dynamic_import_then_source_arguments<'a>(
        &self,
        callee: &'a Expression,
    ) -> Option<&'a [CallArgument]> {
        let Expression::Member { object, property } = callee else {
            return None;
        };
        if !matches!(property.as_ref(), Expression::String(name) if name == "then") {
            return None;
        }
        let Expression::Call {
            callee: import_callee,
            arguments: import_arguments,
        } = object.as_ref()
        else {
            return None;
        };
        if !matches!(import_callee.as_ref(), Expression::Identifier(name) if name == "__ayyDynamicImport")
        {
            return None;
        }
        Some(import_arguments)
    }

    fn static_dynamic_import_module_namespace_object_binding(
        &self,
        arguments: &[CallArgument],
    ) -> Option<ObjectValueBinding> {
        let module_index = self.dynamic_import_literal_module_index(arguments)?;
        self.static_dynamic_import_module_namespace_object_binding_for_index(
            module_index,
            &mut HashSet::new(),
        )
    }

    fn static_dynamic_import_module_namespace_object_binding_for_index(
        &self,
        module_index: usize,
        visited: &mut HashSet<usize>,
    ) -> Option<ObjectValueBinding> {
        if !visited.insert(module_index) {
            return None;
        }
        let Some(init_function) =
            self.registered_function(&format!("__ayy_module_init_{module_index}"))
        else {
            visited.remove(&module_index);
            return None;
        };
        let local_bindings = self.static_dynamic_import_module_local_bindings(init_function);
        let mut binding = empty_object_value_binding();
        binding.extensible = false;

        self.define_static_module_namespace_property(
            &mut binding,
            Expression::Member {
                object: Box::new(Expression::Identifier("Symbol".to_string())),
                property: Box::new(Expression::String("toStringTag".to_string())),
            },
            Expression::String("Module".to_string()),
            false,
            false,
        );
        self.define_static_module_namespace_property(
            &mut binding,
            Expression::String("__ayy$module$namespace".to_string()),
            Expression::Bool(true),
            false,
            false,
        );
        self.define_static_module_namespace_property(
            &mut binding,
            Expression::String(MODULE_NAMESPACE_DESCRIPTOR_MODULE_INDEX.to_string()),
            Expression::Number(module_index as f64),
            false,
            false,
        );

        for statement in &init_function.body {
            let Some((export_name, value)) =
                self.static_dynamic_import_export_value(statement, &local_bindings, visited)
            else {
                continue;
            };
            if export_name == "__ayy$module$namespace" {
                continue;
            }
            self.define_static_module_namespace_property(
                &mut binding,
                Expression::String(export_name),
                value,
                true,
                true,
            );
        }

        visited.remove(&module_index);
        Some(binding)
    }

    fn dynamic_import_literal_module_index(&self, arguments: &[CallArgument]) -> Option<usize> {
        let argument_expression = match arguments.first()? {
            CallArgument::Expression(expression) => expression,
            CallArgument::Spread(_) => return None,
        };
        let argument = self.materialize_global_expression(argument_expression);
        if let Some(module_index) = Self::static_dynamic_import_numeric_module_index(&argument) {
            return Some(module_index);
        }
        let specifier = self
            .static_dynamic_import_specifier_string(argument_expression)
            .or_else(|| self.static_dynamic_import_specifier_string(&argument))?;
        self.dynamic_import_module_index_from_specifier_table(arguments, &specifier)
    }

    fn static_dynamic_import_numeric_module_index(argument: &Expression) -> Option<usize> {
        let Expression::Number(module_index) = argument else {
            return None;
        };
        if !module_index.is_finite() || *module_index < 0.0 || module_index.fract() != 0.0 {
            return None;
        }
        Some(*module_index as usize)
    }

    fn dynamic_import_module_index_from_specifier_table(
        &self,
        arguments: &[CallArgument],
        specifier: &str,
    ) -> Option<usize> {
        let table = match arguments.get(2)? {
            CallArgument::Expression(expression) => self.materialize_global_expression(expression),
            CallArgument::Spread(_) => return None,
        };
        let Expression::Object(entries) = table else {
            return None;
        };
        entries.iter().find_map(|entry| {
            let ObjectEntry::Data { key, value } = entry else {
                return None;
            };
            let key_text = static_property_name_from_expression(key)?;
            if key_text != specifier {
                return None;
            }
            let value = self.materialize_global_expression(value);
            Self::static_dynamic_import_numeric_module_index(&value)
        })
    }

    fn static_dynamic_import_specifier_string(&self, expression: &Expression) -> Option<String> {
        let materialized = self.materialize_global_expression(expression);
        if let Some(text) = Self::static_dynamic_import_primitive_to_string(&materialized) {
            return Some(text);
        }
        let object_binding = self
            .infer_global_object_binding(expression)
            .or_else(|| self.infer_global_object_binding(&materialized))?;
        for method_name in ["toString", "valueOf"] {
            let Some(method) = object_binding_lookup_value(
                &object_binding,
                &Expression::String(method_name.to_string()),
            ) else {
                continue;
            };
            let Some(result) = self.infer_static_call_result_expression(method, &[]) else {
                continue;
            };
            let result = self.materialize_global_expression(&result);
            if let Some(text) = Self::static_dynamic_import_primitive_to_string(&result) {
                return Some(text);
            }
        }
        None
    }

    fn static_dynamic_import_primitive_to_string(expression: &Expression) -> Option<String> {
        match expression {
            Expression::String(text) => Some(text.clone()),
            Expression::Number(value) => {
                if value.is_nan() {
                    Some("NaN".to_string())
                } else if value.is_infinite() {
                    Some(
                        if value.is_sign_positive() {
                            "Infinity"
                        } else {
                            "-Infinity"
                        }
                        .to_string(),
                    )
                } else if *value == 0.0 {
                    Some("0".to_string())
                } else if value.fract() == 0.0 {
                    Some((*value as i64).to_string())
                } else {
                    Some(value.to_string())
                }
            }
            Expression::Bool(value) => Some(value.to_string()),
            Expression::Null => Some("null".to_string()),
            Expression::Undefined => Some("undefined".to_string()),
            Expression::BigInt(value) => Some(value.trim_end_matches('n').to_string()),
            _ => None,
        }
    }

    fn define_static_module_namespace_property(
        &self,
        binding: &mut ObjectValueBinding,
        property: Expression,
        value: Expression,
        enumerable: bool,
        writable: bool,
    ) {
        object_binding_define_property_descriptor(
            binding,
            property,
            PropertyDescriptorBinding {
                value: Some(value),
                configurable: false,
                enumerable,
                writable: Some(writable),
                getter: None,
                setter: None,
                has_get: false,
                has_set: false,
            },
        );
    }

    fn static_dynamic_import_module_local_bindings(
        &self,
        init_function: &FunctionDeclaration,
    ) -> HashMap<String, Expression> {
        let mut bindings = HashMap::new();
        for statement in &init_function.body {
            self.collect_static_dynamic_import_module_local_bindings(statement, &mut bindings);
        }
        bindings
    }

    fn collect_static_dynamic_import_module_local_bindings(
        &self,
        statement: &Statement,
        bindings: &mut HashMap<String, Expression>,
    ) {
        match statement {
            Statement::Var { name, value }
            | Statement::Let { name, value, .. }
            | Statement::Assign { name, value } => {
                bindings.insert(name.clone(), self.materialize_global_expression(value));
            }
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => {
                for statement in body {
                    self.collect_static_dynamic_import_module_local_bindings(statement, bindings);
                }
            }
            _ => {}
        }
    }

    fn static_dynamic_import_local_binding_value(
        name: &str,
        local_bindings: &HashMap<String, Expression>,
    ) -> Option<Expression> {
        let mut current = name.to_string();
        let mut visited = HashSet::new();
        loop {
            if !visited.insert(current.clone()) {
                return None;
            }
            let value = local_bindings.get(&current)?.clone();
            let Expression::Identifier(alias) = &value else {
                return Some(value);
            };
            if !local_bindings.contains_key(alias) {
                return Some(value);
            }
            current = alias.clone();
        }
    }

    fn static_dynamic_import_export_value(
        &self,
        statement: &Statement,
        local_bindings: &HashMap<String, Expression>,
        visited: &mut HashSet<usize>,
    ) -> Option<(String, Expression)> {
        let Statement::Expression(Expression::Call { callee, arguments }) = statement else {
            return None;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return None;
        };
        if !matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
            || !matches!(property.as_ref(), Expression::String(name) if name == "defineProperty")
        {
            return None;
        }
        let [
            CallArgument::Expression(Expression::Identifier(exports_name)),
            CallArgument::Expression(Expression::String(export_name)),
            CallArgument::Expression(Expression::Object(descriptor_entries)),
            ..,
        ] = arguments.as_slice()
        else {
            return None;
        };
        if exports_name != "exports" || export_name.starts_with("__ayy$") {
            return None;
        }

        if let Some(module_index) =
            static_module_namespace_descriptor_module_index(descriptor_entries)
        {
            let namespace_binding = self
                .static_dynamic_import_module_namespace_object_binding_for_index(
                    module_index,
                    visited,
                )?;
            return Some((
                export_name.clone(),
                object_binding_to_expression(&namespace_binding),
            ));
        }

        if let Some((module_index, imported_name)) =
            static_module_reexport_descriptor_source(descriptor_entries)
        {
            let namespace_binding = self
                .static_dynamic_import_module_namespace_object_binding_for_index(
                    module_index,
                    visited,
                )?;
            let value =
                object_binding_lookup_value(&namespace_binding, &Expression::String(imported_name))
                    .cloned()?;
            return Some((export_name.clone(), value));
        }

        let value = descriptor_entries
            .iter()
            .find_map(|entry| match entry {
                ObjectEntry::Data { key, value }
                    if matches!(key, Expression::String(name) if name == "value") =>
                {
                    Some(self.materialize_global_expression(value))
                }
                ObjectEntry::Data { key, value }
                    if matches!(key, Expression::String(name) if name == "get") =>
                {
                    let Expression::Identifier(getter_name) = value else {
                        return None;
                    };
                    self.static_dynamic_import_getter_value(getter_name, local_bindings)
                }
                _ => None,
            })
            .unwrap_or(Expression::Undefined);
        if self.expression_references_module_dependency_param(&value) {
            return None;
        }
        Some((export_name.clone(), value))
    }

    fn static_dynamic_import_getter_value(
        &self,
        getter_name: &str,
        local_bindings: &HashMap<String, Expression>,
    ) -> Option<Expression> {
        let getter = self.registered_function(getter_name)?;
        let [Statement::Return(return_value)] = getter.body.as_slice() else {
            return None;
        };
        Some(match return_value {
            Expression::Identifier(name) => {
                let value = Self::static_dynamic_import_local_binding_value(name, local_bindings)
                    .unwrap_or_else(|| return_value.clone());
                self.static_dynamic_import_live_binding_value(name, &value, getter_name)
                    .unwrap_or(value)
            }
            Expression::Member { object, property } if matches!(object.as_ref(), Expression::Identifier(name) if name.starts_with("__ayy_module_dep_")) =>
            {
                let Expression::String(property_name) = property.as_ref() else {
                    return Some(self.materialize_global_expression(return_value));
                };
                let value =
                    Self::static_dynamic_import_local_binding_value(property_name, local_bindings)
                        .unwrap_or_else(|| self.materialize_global_expression(return_value));
                self.static_dynamic_import_live_binding_value(property_name, &value, getter_name)
                    .unwrap_or(value)
            }
            _ => self.materialize_global_expression(return_value),
        })
    }

    fn static_dynamic_import_live_binding_value(
        &self,
        binding_name: &str,
        value: &Expression,
        getter_name: &str,
    ) -> Option<Expression> {
        if let Expression::Identifier(function_name) = value
            && let Some(hidden_name) = self
                .state
                .user_function_capture_bindings(function_name)
                .and_then(|captures| captures.get(binding_name).cloned())
        {
            return Some(Expression::Identifier(hidden_name));
        }

        let mut getter_capture = None;
        for user_function in self.state.user_functions() {
            let Some(hidden_name) = self
                .state
                .user_function_capture_bindings(&user_function.name)
                .and_then(|captures| captures.get(binding_name).cloned())
            else {
                continue;
            };
            if user_function.name == getter_name {
                getter_capture = Some(Expression::Identifier(hidden_name));
            } else {
                return Some(Expression::Identifier(hidden_name));
            }
        }

        getter_capture
    }

    fn expression_references_module_dependency_param(&self, expression: &Expression) -> bool {
        match expression {
            Expression::Identifier(name) => name.starts_with("__ayy_module_dep_"),
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                    self.expression_references_module_dependency_param(expression)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    self.expression_references_module_dependency_param(key)
                        || self.expression_references_module_dependency_param(value)
                }
                ObjectEntry::Getter { key, getter } => {
                    self.expression_references_module_dependency_param(key)
                        || self.expression_references_module_dependency_param(getter)
                }
                ObjectEntry::Setter { key, setter } => {
                    self.expression_references_module_dependency_param(key)
                        || self.expression_references_module_dependency_param(setter)
                }
                ObjectEntry::Spread(expression) => {
                    self.expression_references_module_dependency_param(expression)
                }
            }),
            Expression::Member { object, property } => {
                self.expression_references_module_dependency_param(object)
                    || self.expression_references_module_dependency_param(property)
            }
            Expression::SuperMember { property } => {
                self.expression_references_module_dependency_param(property)
            }
            Expression::Assign { value, .. }
            | Expression::AssignMember { value, .. }
            | Expression::AssignSuperMember { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => self.expression_references_module_dependency_param(value),
            Expression::Binary { left, right, .. } => {
                self.expression_references_module_dependency_param(left)
                    || self.expression_references_module_dependency_param(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.expression_references_module_dependency_param(condition)
                    || self.expression_references_module_dependency_param(then_expression)
                    || self.expression_references_module_dependency_param(else_expression)
            }
            Expression::Sequence(expressions) => expressions
                .iter()
                .any(|expression| self.expression_references_module_dependency_param(expression)),
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                self.expression_references_module_dependency_param(callee)
                    || arguments.iter().any(|argument| {
                        self.expression_references_module_dependency_param(argument.expression())
                    })
            }
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::NewTarget
            | Expression::This
            | Expression::Sent
            | Expression::Update { .. } => false,
        }
    }
}

fn static_module_namespace_descriptor_module_index(entries: &[ObjectEntry]) -> Option<usize> {
    entries.iter().find_map(|entry| match entry {
        ObjectEntry::Data {
            key: Expression::String(key),
            value: Expression::Number(index),
        } if key == MODULE_NAMESPACE_DESCRIPTOR_MODULE_INDEX
            && index.is_finite()
            && *index >= 0.0
            && index.fract() == 0.0 =>
        {
            Some(*index as usize)
        }
        _ => None,
    })
}

fn static_module_reexport_descriptor_source(entries: &[ObjectEntry]) -> Option<(usize, String)> {
    let module_index = entries.iter().find_map(|entry| match entry {
        ObjectEntry::Data {
            key: Expression::String(key),
            value: Expression::Number(index),
        } if key == MODULE_REEXPORT_DESCRIPTOR_MODULE_INDEX
            && index.is_finite()
            && *index >= 0.0
            && index.fract() == 0.0 =>
        {
            Some(*index as usize)
        }
        _ => None,
    })?;
    let imported_name = entries.iter().find_map(|entry| match entry {
        ObjectEntry::Data {
            key: Expression::String(key),
            value: Expression::String(name),
        } if key == MODULE_REEXPORT_DESCRIPTOR_NAME => Some(name.clone()),
        _ => None,
    })?;
    Some((module_index, imported_name))
}
