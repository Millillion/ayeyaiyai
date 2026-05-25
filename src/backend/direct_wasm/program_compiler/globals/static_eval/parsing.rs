use super::*;

impl DirectWasmCompiler {
    fn eval_internal_function_name_hint(function_name: &str) -> Option<&str> {
        function_name
            .rsplit_once("__name_")
            .map(|(_, hinted_name)| hinted_name)
            .filter(|hinted_name| !hinted_name.is_empty())
    }

    fn class_binding_name_for_static_eval_function(&self, function_name: &str) -> Option<String> {
        if function_name.starts_with("__ayy_class_ctor_") {
            return self
                .registered_function(function_name)
                .and_then(|declaration| declaration.self_binding.clone())
                .or_else(|| {
                    Self::eval_internal_function_name_hint(function_name).map(str::to_string)
                });
        }

        let home_object_name = self.resolve_home_object_name_for_function_static(function_name)?;
        Some(
            home_object_name
                .strip_suffix(".prototype")
                .unwrap_or(home_object_name.as_str())
                .to_string(),
        )
    }

    fn collect_static_eval_private_name_from_expression(
        expression: &Expression,
        prefix: &str,
        names: &mut Vec<String>,
    ) {
        if let Expression::String(property_name) = expression
            && let Some(name) = property_name.strip_prefix(prefix)
        {
            names.push(name.to_string());
        }
    }

    fn collect_static_eval_private_names_from_statement(
        statement: &Statement,
        prefix: &str,
        names: &mut Vec<String>,
    ) {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => {
                for statement in body {
                    Self::collect_static_eval_private_names_from_statement(
                        statement, prefix, names,
                    );
                }
            }
            Statement::AssignMember {
                object, property, ..
            } if matches!(object, Expression::This) => {
                Self::collect_static_eval_private_name_from_expression(property, prefix, names);
            }
            Statement::Expression(Expression::Call { callee, arguments })
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
                    ..,
                ] = arguments.as_slice()
                else {
                    return;
                };
                if matches!(target, Expression::This) {
                    Self::collect_static_eval_private_name_from_expression(property, prefix, names);
                }
            }
            _ => {}
        }
    }

    fn class_private_names_for_static_eval_function(
        &self,
        function_name: &str,
    ) -> Option<(String, Vec<String>)> {
        let class_binding_name = self.class_binding_name_for_static_eval_function(function_name)?;
        let prefix = format!("__ayy$private${class_binding_name}$");
        let prototype_binding_name = format!("{class_binding_name}.prototype");
        let mut private_names = Vec::new();

        for user_function in self.state.user_functions() {
            let belongs_to_class = user_function.home_object_binding.as_deref()
                == Some(class_binding_name.as_str())
                || user_function.home_object_binding.as_deref()
                    == Some(prototype_binding_name.as_str())
                || self
                    .registered_function(&user_function.name)
                    .and_then(|declaration| declaration.self_binding.as_deref())
                    == Some(class_binding_name.as_str());
            if !belongs_to_class {
                continue;
            }
            if let Some(declaration) = self.registered_function(&user_function.name) {
                for statement in &declaration.body {
                    Self::collect_static_eval_private_names_from_statement(
                        statement,
                        &prefix,
                        &mut private_names,
                    );
                }
            }
        }

        private_names.sort();
        private_names.dedup();
        Some((class_binding_name, private_names))
    }

    fn parse_eval_program_in_class_field_initializer_context_static_for_function(
        &self,
        function_name: &str,
        source: &str,
    ) -> Option<Program> {
        let (class_binding_name, private_names) =
            self.class_private_names_for_static_eval_function(function_name)?;
        let wrapper_method_name = "__ayy_eval_wrapper__";
        let private_declarations = private_names
            .into_iter()
            .map(|name| format!("#{name};"))
            .collect::<Vec<_>>()
            .join("\n");
        let wrapped_source = if private_declarations.is_empty() {
            format!("class {class_binding_name} {{\n{wrapper_method_name}() {{\n{source}\n}}\n}}")
        } else {
            format!(
                "class {class_binding_name} {{\n{private_declarations}\n{wrapper_method_name}() {{\n{source}\n}}\n}}"
            )
        };
        let mut wrapped_program = frontend::parse(&wrapped_source).ok()?;
        let wrapper_function = wrapped_program
            .functions
            .iter()
            .find(|function| {
                Self::eval_internal_function_name_hint(&function.name) == Some(wrapper_method_name)
            })
            .cloned()
            .or_else(|| {
                let wrapper_methods = wrapped_program
                    .functions
                    .iter()
                    .filter(|function| function.name.starts_with("__ayy_class_method_"))
                    .cloned()
                    .collect::<Vec<_>>();
                (wrapper_methods.len() == 1).then(|| wrapper_methods[0].clone())
            })?;
        let wrapper_class_binding_name = wrapper_function
            .immutable_class_bindings
            .first()
            .cloned()
            .or_else(|| wrapper_function.private_brand_binding.clone())
            .or_else(|| wrapper_function.self_binding.clone())
            .unwrap_or_else(|| class_binding_name.clone());

        wrapped_program.functions.retain(|function| {
            if function.name == wrapper_function.name {
                return false;
            }
            if function.name.starts_with("__ayy_class_ctor_")
                && Self::eval_internal_function_name_hint(&function.name).is_some_and(|hint| {
                    hint == class_binding_name || hint == wrapper_class_binding_name
                })
            {
                return false;
            }
            if function.name.starts_with("__ayy_class_init_")
                && function.body.iter().any(|statement| {
                    matches!(
                        statement,
                        Statement::Return(Expression::Identifier(name))
                            if name == &class_binding_name || name == &wrapper_class_binding_name
                    )
                })
            {
                return false;
            }
            true
        });

        Some(Program {
            strict: wrapper_function.strict,
            functions: wrapped_program.functions,
            statements: wrapper_function.body,
        })
    }

    pub(in crate::backend::direct_wasm) fn parse_static_eval_program_in_context(
        &self,
        source: &str,
        current_function_name: Option<&str>,
    ) -> Option<Program> {
        if frontend::script_goal_has_direct_using_declaration(source) {
            return None;
        }

        if let Some(current_function_name) = current_function_name {
            if self
                .registered_function(current_function_name)
                .is_some_and(|function| function.direct_eval_in_class_field_initializer)
                && let Some(program) = self
                    .parse_eval_program_in_class_field_initializer_context_static_for_function(
                        current_function_name,
                        source,
                    )
            {
                return Some(program);
            }
            if self
                .resolve_home_object_name_for_function_static(current_function_name)
                .is_some()
                && source.contains("super")
                && let Some(program) = self.parse_eval_program_in_method_context_static(source)
            {
                return Some(program);
            }
            if let Some(program) =
                self.parse_eval_program_in_ordinary_function_context_static(source)
            {
                return Some(program);
            }
        }
        frontend::parse(source).ok()
    }

    pub(in crate::backend::direct_wasm) fn resolve_home_object_name_for_function_static(
        &self,
        function_name: &str,
    ) -> Option<String> {
        if let Some(home_object_name) = self.user_function_home_object_binding(function_name) {
            return Some(home_object_name);
        }
        self.find_global_home_object_binding_name(function_name)
    }

    pub(in crate::backend::direct_wasm) fn parse_eval_program_in_ordinary_function_context_static(
        &self,
        source: &str,
    ) -> Option<Program> {
        let wrapper_name = "__ayy_eval_new_target_context__";
        let wrapped_source = format!("function {wrapper_name}() {{\n{source}\n}}");
        let mut wrapped_program = frontend::parse(&wrapped_source).ok()?;
        let wrapper = wrapped_program
            .functions
            .iter()
            .find(|function| function.name == wrapper_name)
            .cloned()?;
        wrapped_program
            .functions
            .retain(|function| function.name != wrapper_name);

        Some(Program {
            strict: wrapper.strict,
            functions: wrapped_program.functions,
            statements: wrapper.body,
        })
    }

    pub(in crate::backend::direct_wasm) fn parse_eval_program_in_method_context_static(
        &self,
        source: &str,
    ) -> Option<Program> {
        let wrapper_property = "__ayy_eval_wrapper__";
        let wrapped_source = format!("({{{wrapper_property}() {{\n{source}\n}}}});");
        let mut wrapped_program = frontend::parse(&wrapped_source).ok()?;
        let wrapper_name = wrapped_program.statements.iter().find_map(|statement| {
            let Statement::Expression(Expression::Object(entries)) = statement else {
                return None;
            };
            entries.iter().find_map(|entry| match entry {
                crate::ir::hir::ObjectEntry::Data { key, value }
                    if matches!(key, Expression::String(name) if name == wrapper_property) =>
                {
                    let Expression::Identifier(name) = value else {
                        return None;
                    };
                    Some(name.clone())
                }
                _ => None,
            })
        })?;
        let wrapper = wrapped_program
            .functions
            .iter()
            .find(|function| function.name == wrapper_name)
            .cloned()?;
        wrapped_program
            .functions
            .retain(|function| function.name != wrapper_name);

        Some(Program {
            strict: wrapper.strict,
            functions: wrapped_program.functions,
            statements: wrapper.body,
        })
    }
}
