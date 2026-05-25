use super::*;

impl<'a> FunctionCompiler<'a> {
    fn eval_internal_function_name_hint(function_name: &str) -> Option<&str> {
        function_name
            .rsplit_once("__name_")
            .map(|(_, hinted_name)| hinted_name)
            .filter(|hinted_name| !hinted_name.is_empty())
    }

    pub(in crate::backend::direct_wasm) fn class_binding_name_for_function(
        &self,
        function_name: &str,
    ) -> Option<String> {
        if function_name.starts_with("__ayy_class_ctor_") {
            return self
                .resolve_registered_function_declaration(function_name)
                .and_then(|declaration| declaration.self_binding.clone())
                .or_else(|| {
                    Self::eval_internal_function_name_hint(function_name).map(str::to_string)
                });
        }

        if function_name.starts_with("__ayy_class_init_") {
            return self
                .resolve_registered_function_declaration(function_name)
                .and_then(|declaration| {
                    declaration
                        .body
                        .iter()
                        .rev()
                        .find_map(|statement| match statement {
                            Statement::Return(Expression::Identifier(name)) => Some(name.clone()),
                            _ => None,
                        })
                });
        }

        let home_object_name = self.resolve_home_object_name_for_function(function_name)?;
        Some(
            home_object_name
                .strip_suffix(".prototype")
                .unwrap_or(home_object_name.as_str())
                .to_string(),
        )
    }

    fn class_private_names_for_function(
        &self,
        function_name: &str,
    ) -> Option<(String, Vec<String>)> {
        let class_binding_name = self.class_binding_name_for_function(function_name)?;
        let prefix = format!("__ayy$private${class_binding_name}$");
        let mut private_names = self
            .current_function_name()
            .filter(|current| *current == function_name)
            .and_then(|_| self.resolve_object_binding_from_expression(&Expression::This))
            .map(|this_binding| {
                this_binding
                    .string_properties
                    .iter()
                    .filter_map(|(property_name, _)| {
                        property_name.strip_prefix(&prefix).map(str::to_string)
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        Self::collect_private_names_from_identifier_binding(
            self,
            &class_binding_name,
            &prefix,
            &mut private_names,
        );
        let prototype_binding_name = format!("{class_binding_name}.prototype");
        Self::collect_private_names_from_identifier_binding(
            self,
            &prototype_binding_name,
            &prefix,
            &mut private_names,
        );
        for user_function in self.user_functions() {
            let belongs_to_class = user_function.home_object_binding.as_deref()
                == Some(class_binding_name.as_str())
                || user_function.home_object_binding.as_deref()
                    == Some(prototype_binding_name.as_str())
                || self
                    .resolve_registered_function_declaration(&user_function.name)
                    .and_then(|declaration| declaration.self_binding.as_deref())
                    == Some(class_binding_name.as_str());
            if !belongs_to_class {
                continue;
            }
            if let Some(declaration) =
                self.resolve_registered_function_declaration(&user_function.name)
            {
                for statement in &declaration.body {
                    Self::collect_private_names_from_statement(
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

    fn collect_private_names_from_identifier_binding(
        &self,
        binding_name: &str,
        prefix: &str,
        names: &mut Vec<String>,
    ) {
        let Some(binding) = self.resolve_object_binding_from_expression(&Expression::Identifier(
            binding_name.to_string(),
        )) else {
            return;
        };
        names.extend(
            binding
                .string_properties
                .iter()
                .filter_map(|(property_name, _)| {
                    property_name.strip_prefix(prefix).map(str::to_string)
                }),
        );
    }

    fn collect_private_names_from_statement(
        statement: &Statement,
        prefix: &str,
        names: &mut Vec<String>,
    ) {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => {
                for statement in body {
                    Self::collect_private_names_from_statement(statement, prefix, names);
                }
            }
            Statement::AssignMember {
                object, property, ..
            } if matches!(object, Expression::This) => {
                Self::collect_private_name_from_expression(property, prefix, names);
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
                    Self::collect_private_name_from_expression(property, prefix, names);
                }
            }
            _ => {}
        }
    }

    fn collect_private_name_from_expression(
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

    fn rewrite_class_field_eval_wrapper_binding_name(
        name: &mut String,
        wrapper_class_binding_name: &str,
        class_binding_name: &str,
        wrapper_private_brand: Option<&str>,
        class_private_brand: Option<&str>,
    ) {
        if name == wrapper_class_binding_name {
            *name = class_binding_name.to_string();
            return;
        }
        if let (Some(wrapper_private_brand), Some(class_private_brand)) =
            (wrapper_private_brand, class_private_brand)
            && name == wrapper_private_brand
        {
            *name = class_private_brand.to_string();
        }
    }

    fn rewrite_class_field_eval_wrapper_private_property_name(
        name: &mut String,
        wrapper_class_binding_name: &str,
        class_binding_name: &str,
    ) {
        let wrapper_prefix = format!("__ayy$private${wrapper_class_binding_name}$");
        if let Some(private_name) = name.strip_prefix(&wrapper_prefix) {
            *name = format!("__ayy$private${class_binding_name}${private_name}");
        }
    }

    fn rewrite_class_field_eval_wrapper_private_expression(
        expression: &mut Expression,
        wrapper_class_binding_name: &str,
        class_binding_name: &str,
        wrapper_private_brand: Option<&str>,
        class_private_brand: Option<&str>,
        property_position: bool,
    ) {
        match expression {
            Expression::Identifier(name) | Expression::Update { name, .. } => {
                Self::rewrite_class_field_eval_wrapper_binding_name(
                    name,
                    wrapper_class_binding_name,
                    class_binding_name,
                    wrapper_private_brand,
                    class_private_brand,
                );
            }
            Expression::String(name) if property_position => {
                Self::rewrite_class_field_eval_wrapper_private_property_name(
                    name,
                    wrapper_class_binding_name,
                    class_binding_name,
                );
            }
            Expression::Member { object, property } => {
                Self::rewrite_class_field_eval_wrapper_private_expression(
                    object,
                    wrapper_class_binding_name,
                    class_binding_name,
                    wrapper_private_brand,
                    class_private_brand,
                    false,
                );
                Self::rewrite_class_field_eval_wrapper_private_expression(
                    property,
                    wrapper_class_binding_name,
                    class_binding_name,
                    wrapper_private_brand,
                    class_private_brand,
                    true,
                );
            }
            Expression::SuperMember { property } => {
                Self::rewrite_class_field_eval_wrapper_private_expression(
                    property,
                    wrapper_class_binding_name,
                    class_binding_name,
                    wrapper_private_brand,
                    class_private_brand,
                    true,
                );
            }
            Expression::Assign { name, value } => {
                Self::rewrite_class_field_eval_wrapper_binding_name(
                    name,
                    wrapper_class_binding_name,
                    class_binding_name,
                    wrapper_private_brand,
                    class_private_brand,
                );
                Self::rewrite_class_field_eval_wrapper_private_expression(
                    value,
                    wrapper_class_binding_name,
                    class_binding_name,
                    wrapper_private_brand,
                    class_private_brand,
                    false,
                );
            }
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                Self::rewrite_class_field_eval_wrapper_private_expression(
                    object,
                    wrapper_class_binding_name,
                    class_binding_name,
                    wrapper_private_brand,
                    class_private_brand,
                    false,
                );
                Self::rewrite_class_field_eval_wrapper_private_expression(
                    property,
                    wrapper_class_binding_name,
                    class_binding_name,
                    wrapper_private_brand,
                    class_private_brand,
                    true,
                );
                Self::rewrite_class_field_eval_wrapper_private_expression(
                    value,
                    wrapper_class_binding_name,
                    class_binding_name,
                    wrapper_private_brand,
                    class_private_brand,
                    false,
                );
            }
            Expression::AssignSuperMember { property, value } => {
                Self::rewrite_class_field_eval_wrapper_private_expression(
                    property,
                    wrapper_class_binding_name,
                    class_binding_name,
                    wrapper_private_brand,
                    class_private_brand,
                    true,
                );
                Self::rewrite_class_field_eval_wrapper_private_expression(
                    value,
                    wrapper_class_binding_name,
                    class_binding_name,
                    wrapper_private_brand,
                    class_private_brand,
                    false,
                );
            }
            Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => Self::rewrite_class_field_eval_wrapper_private_expression(
                value,
                wrapper_class_binding_name,
                class_binding_name,
                wrapper_private_brand,
                class_private_brand,
                false,
            ),
            Expression::Binary { left, right, .. } => {
                Self::rewrite_class_field_eval_wrapper_private_expression(
                    left,
                    wrapper_class_binding_name,
                    class_binding_name,
                    wrapper_private_brand,
                    class_private_brand,
                    false,
                );
                Self::rewrite_class_field_eval_wrapper_private_expression(
                    right,
                    wrapper_class_binding_name,
                    class_binding_name,
                    wrapper_private_brand,
                    class_private_brand,
                    false,
                );
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::rewrite_class_field_eval_wrapper_private_expression(
                    condition,
                    wrapper_class_binding_name,
                    class_binding_name,
                    wrapper_private_brand,
                    class_private_brand,
                    false,
                );
                Self::rewrite_class_field_eval_wrapper_private_expression(
                    then_expression,
                    wrapper_class_binding_name,
                    class_binding_name,
                    wrapper_private_brand,
                    class_private_brand,
                    false,
                );
                Self::rewrite_class_field_eval_wrapper_private_expression(
                    else_expression,
                    wrapper_class_binding_name,
                    class_binding_name,
                    wrapper_private_brand,
                    class_private_brand,
                    false,
                );
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    Self::rewrite_class_field_eval_wrapper_private_expression(
                        expression,
                        wrapper_class_binding_name,
                        class_binding_name,
                        wrapper_private_brand,
                        class_private_brand,
                        false,
                    );
                }
            }
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                Self::rewrite_class_field_eval_wrapper_private_expression(
                    callee,
                    wrapper_class_binding_name,
                    class_binding_name,
                    wrapper_private_brand,
                    class_private_brand,
                    false,
                );
                for argument in arguments {
                    Self::rewrite_class_field_eval_wrapper_private_expression(
                        argument.expression_mut(),
                        wrapper_class_binding_name,
                        class_binding_name,
                        wrapper_private_brand,
                        class_private_brand,
                        false,
                    );
                }
            }
            Expression::Array(elements) => {
                for element in elements {
                    match element {
                        ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                            Self::rewrite_class_field_eval_wrapper_private_expression(
                                expression,
                                wrapper_class_binding_name,
                                class_binding_name,
                                wrapper_private_brand,
                                class_private_brand,
                                false,
                            );
                        }
                    }
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            Self::rewrite_class_field_eval_wrapper_private_expression(
                                key,
                                wrapper_class_binding_name,
                                class_binding_name,
                                wrapper_private_brand,
                                class_private_brand,
                                true,
                            );
                            Self::rewrite_class_field_eval_wrapper_private_expression(
                                value,
                                wrapper_class_binding_name,
                                class_binding_name,
                                wrapper_private_brand,
                                class_private_brand,
                                false,
                            );
                        }
                        ObjectEntry::Getter { key, getter } => {
                            Self::rewrite_class_field_eval_wrapper_private_expression(
                                key,
                                wrapper_class_binding_name,
                                class_binding_name,
                                wrapper_private_brand,
                                class_private_brand,
                                true,
                            );
                            Self::rewrite_class_field_eval_wrapper_private_expression(
                                getter,
                                wrapper_class_binding_name,
                                class_binding_name,
                                wrapper_private_brand,
                                class_private_brand,
                                false,
                            );
                        }
                        ObjectEntry::Setter { key, setter } => {
                            Self::rewrite_class_field_eval_wrapper_private_expression(
                                key,
                                wrapper_class_binding_name,
                                class_binding_name,
                                wrapper_private_brand,
                                class_private_brand,
                                true,
                            );
                            Self::rewrite_class_field_eval_wrapper_private_expression(
                                setter,
                                wrapper_class_binding_name,
                                class_binding_name,
                                wrapper_private_brand,
                                class_private_brand,
                                false,
                            );
                        }
                        ObjectEntry::Spread(expression) => {
                            Self::rewrite_class_field_eval_wrapper_private_expression(
                                expression,
                                wrapper_class_binding_name,
                                class_binding_name,
                                wrapper_private_brand,
                                class_private_brand,
                                false,
                            );
                        }
                    }
                }
            }
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::NewTarget
            | Expression::This
            | Expression::Sent => {}
        }
    }

    fn rewrite_class_field_eval_wrapper_private_statement(
        statement: &mut Statement,
        wrapper_class_binding_name: &str,
        class_binding_name: &str,
        wrapper_private_brand: Option<&str>,
        class_private_brand: Option<&str>,
    ) {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => {
                for statement in body {
                    Self::rewrite_class_field_eval_wrapper_private_statement(
                        statement,
                        wrapper_class_binding_name,
                        class_binding_name,
                        wrapper_private_brand,
                        class_private_brand,
                    );
                }
            }
            Statement::Var { name, value }
            | Statement::Let { name, value, .. }
            | Statement::Assign { name, value } => {
                Self::rewrite_class_field_eval_wrapper_binding_name(
                    name,
                    wrapper_class_binding_name,
                    class_binding_name,
                    wrapper_private_brand,
                    class_private_brand,
                );
                Self::rewrite_class_field_eval_wrapper_private_expression(
                    value,
                    wrapper_class_binding_name,
                    class_binding_name,
                    wrapper_private_brand,
                    class_private_brand,
                    false,
                );
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                Self::rewrite_class_field_eval_wrapper_private_expression(
                    object,
                    wrapper_class_binding_name,
                    class_binding_name,
                    wrapper_private_brand,
                    class_private_brand,
                    false,
                );
                Self::rewrite_class_field_eval_wrapper_private_expression(
                    property,
                    wrapper_class_binding_name,
                    class_binding_name,
                    wrapper_private_brand,
                    class_private_brand,
                    true,
                );
                Self::rewrite_class_field_eval_wrapper_private_expression(
                    value,
                    wrapper_class_binding_name,
                    class_binding_name,
                    wrapper_private_brand,
                    class_private_brand,
                    false,
                );
            }
            Statement::Expression(expression)
            | Statement::Throw(expression)
            | Statement::Return(expression)
            | Statement::Yield { value: expression }
            | Statement::YieldDelegate { value: expression } => {
                Self::rewrite_class_field_eval_wrapper_private_expression(
                    expression,
                    wrapper_class_binding_name,
                    class_binding_name,
                    wrapper_private_brand,
                    class_private_brand,
                    false,
                );
            }
            Statement::Print { values } => {
                for value in values {
                    Self::rewrite_class_field_eval_wrapper_private_expression(
                        value,
                        wrapper_class_binding_name,
                        class_binding_name,
                        wrapper_private_brand,
                        class_private_brand,
                        false,
                    );
                }
            }
            Statement::With { object, body } => {
                Self::rewrite_class_field_eval_wrapper_private_expression(
                    object,
                    wrapper_class_binding_name,
                    class_binding_name,
                    wrapper_private_brand,
                    class_private_brand,
                    false,
                );
                for statement in body {
                    Self::rewrite_class_field_eval_wrapper_private_statement(
                        statement,
                        wrapper_class_binding_name,
                        class_binding_name,
                        wrapper_private_brand,
                        class_private_brand,
                    );
                }
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                Self::rewrite_class_field_eval_wrapper_private_expression(
                    condition,
                    wrapper_class_binding_name,
                    class_binding_name,
                    wrapper_private_brand,
                    class_private_brand,
                    false,
                );
                for statement in then_branch.iter_mut().chain(else_branch.iter_mut()) {
                    Self::rewrite_class_field_eval_wrapper_private_statement(
                        statement,
                        wrapper_class_binding_name,
                        class_binding_name,
                        wrapper_private_brand,
                        class_private_brand,
                    );
                }
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                for statement in body
                    .iter_mut()
                    .chain(catch_setup.iter_mut())
                    .chain(catch_body.iter_mut())
                {
                    Self::rewrite_class_field_eval_wrapper_private_statement(
                        statement,
                        wrapper_class_binding_name,
                        class_binding_name,
                        wrapper_private_brand,
                        class_private_brand,
                    );
                }
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                Self::rewrite_class_field_eval_wrapper_private_expression(
                    discriminant,
                    wrapper_class_binding_name,
                    class_binding_name,
                    wrapper_private_brand,
                    class_private_brand,
                    false,
                );
                for case in cases {
                    if let Some(test) = &mut case.test {
                        Self::rewrite_class_field_eval_wrapper_private_expression(
                            test,
                            wrapper_class_binding_name,
                            class_binding_name,
                            wrapper_private_brand,
                            class_private_brand,
                            false,
                        );
                    }
                    for statement in &mut case.body {
                        Self::rewrite_class_field_eval_wrapper_private_statement(
                            statement,
                            wrapper_class_binding_name,
                            class_binding_name,
                            wrapper_private_brand,
                            class_private_brand,
                        );
                    }
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
                    Self::rewrite_class_field_eval_wrapper_private_statement(
                        statement,
                        wrapper_class_binding_name,
                        class_binding_name,
                        wrapper_private_brand,
                        class_private_brand,
                    );
                }
                for expression in condition
                    .iter_mut()
                    .chain(update.iter_mut())
                    .chain(break_hook.iter_mut())
                {
                    Self::rewrite_class_field_eval_wrapper_private_expression(
                        expression,
                        wrapper_class_binding_name,
                        class_binding_name,
                        wrapper_private_brand,
                        class_private_brand,
                        false,
                    );
                }
                for statement in body {
                    Self::rewrite_class_field_eval_wrapper_private_statement(
                        statement,
                        wrapper_class_binding_name,
                        class_binding_name,
                        wrapper_private_brand,
                        class_private_brand,
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
                Self::rewrite_class_field_eval_wrapper_private_expression(
                    condition,
                    wrapper_class_binding_name,
                    class_binding_name,
                    wrapper_private_brand,
                    class_private_brand,
                    false,
                );
                if let Some(break_hook) = break_hook {
                    Self::rewrite_class_field_eval_wrapper_private_expression(
                        break_hook,
                        wrapper_class_binding_name,
                        class_binding_name,
                        wrapper_private_brand,
                        class_private_brand,
                        false,
                    );
                }
                for statement in body {
                    Self::rewrite_class_field_eval_wrapper_private_statement(
                        statement,
                        wrapper_class_binding_name,
                        class_binding_name,
                        wrapper_private_brand,
                        class_private_brand,
                    );
                }
            }
            Statement::Break { .. } | Statement::Continue { .. } => {}
        }
    }

    fn rewrite_class_field_eval_wrapper_private_function(
        function: &mut FunctionDeclaration,
        wrapper_class_binding_name: &str,
        class_binding_name: &str,
        wrapper_private_brand: Option<&str>,
        class_private_brand: Option<&str>,
    ) {
        for parameter in &mut function.params {
            Self::rewrite_class_field_eval_wrapper_binding_name(
                &mut parameter.name,
                wrapper_class_binding_name,
                class_binding_name,
                wrapper_private_brand,
                class_private_brand,
            );
            if let Some(default) = &mut parameter.default {
                Self::rewrite_class_field_eval_wrapper_private_expression(
                    default,
                    wrapper_class_binding_name,
                    class_binding_name,
                    wrapper_private_brand,
                    class_private_brand,
                    false,
                );
            }
        }
        if let Some(binding) = &mut function.top_level_binding {
            Self::rewrite_class_field_eval_wrapper_binding_name(
                binding,
                wrapper_class_binding_name,
                class_binding_name,
                wrapper_private_brand,
                class_private_brand,
            );
        }
        if let Some(binding) = &mut function.self_binding {
            Self::rewrite_class_field_eval_wrapper_binding_name(
                binding,
                wrapper_class_binding_name,
                class_binding_name,
                wrapper_private_brand,
                class_private_brand,
            );
        }
        if let Some(binding) = &mut function.private_brand_binding {
            Self::rewrite_class_field_eval_wrapper_binding_name(
                binding,
                wrapper_class_binding_name,
                class_binding_name,
                wrapper_private_brand,
                class_private_brand,
            );
        }
        for binding in &mut function.synthetic_capture_bindings {
            Self::rewrite_class_field_eval_wrapper_binding_name(
                binding,
                wrapper_class_binding_name,
                class_binding_name,
                wrapper_private_brand,
                class_private_brand,
            );
        }
        function.synthetic_capture_bindings.sort();
        function.synthetic_capture_bindings.dedup();
        for binding in &mut function.immutable_class_bindings {
            Self::rewrite_class_field_eval_wrapper_binding_name(
                binding,
                wrapper_class_binding_name,
                class_binding_name,
                wrapper_private_brand,
                class_private_brand,
            );
        }
        function.immutable_class_bindings.sort();
        function.immutable_class_bindings.dedup();
        for statement in &mut function.body {
            Self::rewrite_class_field_eval_wrapper_private_statement(
                statement,
                wrapper_class_binding_name,
                class_binding_name,
                wrapper_private_brand,
                class_private_brand,
            );
        }
    }

    fn rewrite_class_field_eval_wrapper_private_program(
        statements: &mut [Statement],
        functions: &mut [FunctionDeclaration],
        wrapper_class_binding_name: &str,
        class_binding_name: &str,
        wrapper_private_brand: Option<&str>,
        class_private_brand: Option<&str>,
    ) {
        for statement in statements {
            Self::rewrite_class_field_eval_wrapper_private_statement(
                statement,
                wrapper_class_binding_name,
                class_binding_name,
                wrapper_private_brand,
                class_private_brand,
            );
        }
        for function in functions {
            Self::rewrite_class_field_eval_wrapper_private_function(
                function,
                wrapper_class_binding_name,
                class_binding_name,
                wrapper_private_brand,
                class_private_brand,
            );
        }
    }

    fn rewrite_static_class_field_eval_this_expression(
        expression: &mut Expression,
        class_binding_name: &str,
    ) {
        match expression {
            Expression::This => {
                *expression = Expression::Identifier(class_binding_name.to_string());
            }
            Expression::Member { object, property } => {
                Self::rewrite_static_class_field_eval_this_expression(object, class_binding_name);
                Self::rewrite_static_class_field_eval_this_expression(property, class_binding_name);
            }
            Expression::SuperMember { property } => {
                Self::rewrite_static_class_field_eval_this_expression(property, class_binding_name);
            }
            Expression::Assign { value, .. } => {
                Self::rewrite_static_class_field_eval_this_expression(value, class_binding_name);
            }
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                Self::rewrite_static_class_field_eval_this_expression(object, class_binding_name);
                Self::rewrite_static_class_field_eval_this_expression(property, class_binding_name);
                Self::rewrite_static_class_field_eval_this_expression(value, class_binding_name);
            }
            Expression::AssignSuperMember { property, value } => {
                Self::rewrite_static_class_field_eval_this_expression(property, class_binding_name);
                Self::rewrite_static_class_field_eval_this_expression(value, class_binding_name);
            }
            Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => Self::rewrite_static_class_field_eval_this_expression(value, class_binding_name),
            Expression::Binary { left, right, .. } => {
                Self::rewrite_static_class_field_eval_this_expression(left, class_binding_name);
                Self::rewrite_static_class_field_eval_this_expression(right, class_binding_name);
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::rewrite_static_class_field_eval_this_expression(
                    condition,
                    class_binding_name,
                );
                Self::rewrite_static_class_field_eval_this_expression(
                    then_expression,
                    class_binding_name,
                );
                Self::rewrite_static_class_field_eval_this_expression(
                    else_expression,
                    class_binding_name,
                );
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    Self::rewrite_static_class_field_eval_this_expression(
                        expression,
                        class_binding_name,
                    );
                }
            }
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                Self::rewrite_static_class_field_eval_this_expression(callee, class_binding_name);
                for argument in arguments {
                    Self::rewrite_static_class_field_eval_this_expression(
                        argument.expression_mut(),
                        class_binding_name,
                    );
                }
            }
            Expression::Array(elements) => {
                for element in elements {
                    match element {
                        ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                            Self::rewrite_static_class_field_eval_this_expression(
                                expression,
                                class_binding_name,
                            );
                        }
                    }
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            Self::rewrite_static_class_field_eval_this_expression(
                                key,
                                class_binding_name,
                            );
                            Self::rewrite_static_class_field_eval_this_expression(
                                value,
                                class_binding_name,
                            );
                        }
                        ObjectEntry::Getter { key, getter } => {
                            Self::rewrite_static_class_field_eval_this_expression(
                                key,
                                class_binding_name,
                            );
                            Self::rewrite_static_class_field_eval_this_expression(
                                getter,
                                class_binding_name,
                            );
                        }
                        ObjectEntry::Setter { key, setter } => {
                            Self::rewrite_static_class_field_eval_this_expression(
                                key,
                                class_binding_name,
                            );
                            Self::rewrite_static_class_field_eval_this_expression(
                                setter,
                                class_binding_name,
                            );
                        }
                        ObjectEntry::Spread(expression) => {
                            Self::rewrite_static_class_field_eval_this_expression(
                                expression,
                                class_binding_name,
                            );
                        }
                    }
                }
            }
            Expression::Identifier(_)
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::NewTarget
            | Expression::Sent
            | Expression::Update { .. } => {}
        }
    }

    fn rewrite_static_class_field_eval_this_statement(
        statement: &mut Statement,
        class_binding_name: &str,
    ) {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => {
                for statement in body {
                    Self::rewrite_static_class_field_eval_this_statement(
                        statement,
                        class_binding_name,
                    );
                }
            }
            Statement::Var { value, .. } | Statement::Let { value, .. } => {
                Self::rewrite_static_class_field_eval_this_expression(value, class_binding_name);
            }
            Statement::Assign { value, .. } => {
                Self::rewrite_static_class_field_eval_this_expression(value, class_binding_name);
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                Self::rewrite_static_class_field_eval_this_expression(object, class_binding_name);
                Self::rewrite_static_class_field_eval_this_expression(property, class_binding_name);
                Self::rewrite_static_class_field_eval_this_expression(value, class_binding_name);
            }
            Statement::Expression(expression)
            | Statement::Throw(expression)
            | Statement::Return(expression)
            | Statement::Yield { value: expression }
            | Statement::YieldDelegate { value: expression } => {
                Self::rewrite_static_class_field_eval_this_expression(
                    expression,
                    class_binding_name,
                );
            }
            Statement::Print { values } => {
                for value in values {
                    Self::rewrite_static_class_field_eval_this_expression(
                        value,
                        class_binding_name,
                    );
                }
            }
            Statement::With { object, body } => {
                Self::rewrite_static_class_field_eval_this_expression(object, class_binding_name);
                for statement in body {
                    Self::rewrite_static_class_field_eval_this_statement(
                        statement,
                        class_binding_name,
                    );
                }
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                Self::rewrite_static_class_field_eval_this_expression(
                    condition,
                    class_binding_name,
                );
                for statement in then_branch.iter_mut().chain(else_branch.iter_mut()) {
                    Self::rewrite_static_class_field_eval_this_statement(
                        statement,
                        class_binding_name,
                    );
                }
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                for statement in body
                    .iter_mut()
                    .chain(catch_setup.iter_mut())
                    .chain(catch_body.iter_mut())
                {
                    Self::rewrite_static_class_field_eval_this_statement(
                        statement,
                        class_binding_name,
                    );
                }
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                Self::rewrite_static_class_field_eval_this_expression(
                    discriminant,
                    class_binding_name,
                );
                for case in cases {
                    if let Some(test) = &mut case.test {
                        Self::rewrite_static_class_field_eval_this_expression(
                            test,
                            class_binding_name,
                        );
                    }
                    for statement in &mut case.body {
                        Self::rewrite_static_class_field_eval_this_statement(
                            statement,
                            class_binding_name,
                        );
                    }
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
                    Self::rewrite_static_class_field_eval_this_statement(
                        statement,
                        class_binding_name,
                    );
                }
                for expression in condition
                    .iter_mut()
                    .chain(update.iter_mut())
                    .chain(break_hook.iter_mut())
                {
                    Self::rewrite_static_class_field_eval_this_expression(
                        expression,
                        class_binding_name,
                    );
                }
                for statement in body {
                    Self::rewrite_static_class_field_eval_this_statement(
                        statement,
                        class_binding_name,
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
                Self::rewrite_static_class_field_eval_this_expression(
                    condition,
                    class_binding_name,
                );
                if let Some(break_hook) = break_hook {
                    Self::rewrite_static_class_field_eval_this_expression(
                        break_hook,
                        class_binding_name,
                    );
                }
                for statement in body {
                    Self::rewrite_static_class_field_eval_this_statement(
                        statement,
                        class_binding_name,
                    );
                }
            }
            Statement::Break { .. } | Statement::Continue { .. } => {}
        }
    }

    fn rewrite_static_class_field_eval_this_program(
        statements: &mut [Statement],
        functions: &mut [FunctionDeclaration],
        class_binding_name: &str,
    ) {
        for statement in statements {
            Self::rewrite_static_class_field_eval_this_statement(statement, class_binding_name);
        }
        for function in functions {
            if function.lexical_this {
                for statement in &mut function.body {
                    Self::rewrite_static_class_field_eval_this_statement(
                        statement,
                        class_binding_name,
                    );
                }
            }
        }
    }

    pub(in crate::backend::direct_wasm) fn parse_eval_program_in_class_field_initializer_context(
        &self,
        source: &str,
    ) -> Option<Program> {
        let current_function_name = self.current_function_name()?;
        self.parse_eval_program_in_class_field_initializer_context_for_function(
            current_function_name,
            source,
        )
    }

    pub(in crate::backend::direct_wasm) fn parse_eval_program_in_class_field_initializer_context_for_function(
        &self,
        function_name: &str,
        source: &str,
    ) -> Option<Program> {
        let (class_binding_name, private_names) =
            self.class_private_names_for_function(function_name)?;
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
        let wrapper_private_brand = wrapper_function.private_brand_binding.clone();
        let class_private_brand = self
            .resolve_registered_function_declaration(function_name)
            .and_then(|function| function.private_brand_binding.clone());

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
                            if name == &class_binding_name
                    )
                })
            {
                return false;
            }
            true
        });
        let mut statements = wrapper_function.body;
        Self::rewrite_class_field_eval_wrapper_private_program(
            &mut statements,
            &mut wrapped_program.functions,
            &wrapper_class_binding_name,
            &class_binding_name,
            wrapper_private_brand.as_deref(),
            class_private_brand.as_deref(),
        );
        if function_name.starts_with("__ayy_class_init_") {
            Self::rewrite_static_class_field_eval_this_program(
                &mut statements,
                &mut wrapped_program.functions,
                &class_binding_name,
            );
        }

        Some(Program {
            strict: wrapper_function.strict,
            functions: wrapped_program.functions,
            statements,
        })
    }

    fn parse_eval_program_in_class_method_context_for_function(
        &self,
        function_name: &str,
        source: &str,
    ) -> Option<Program> {
        let (class_binding_name, private_names) =
            self.class_private_names_for_function(function_name)?;
        let user_function = self.user_function(function_name)?;
        let home_object_name = self.resolve_home_object_name_for_function(function_name)?;
        let wrapper_method_name = "__ayy_eval_wrapper__";
        let private_declarations = private_names
            .into_iter()
            .map(|name| format!("#{name};"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut wrapper_method_prefix = String::new();
        if !home_object_name.ends_with(".prototype") {
            wrapper_method_prefix.push_str("static ");
        }
        if user_function.is_async() {
            wrapper_method_prefix.push_str("async ");
        }
        if user_function.is_generator() {
            wrapper_method_prefix.push('*');
        }
        let wrapped_source = if private_declarations.is_empty() {
            format!(
                "class {class_binding_name} {{\n{wrapper_method_prefix}{wrapper_method_name}() {{\n{source}\n}}\n}}"
            )
        } else {
            format!(
                "class {class_binding_name} {{\n{private_declarations}\n{wrapper_method_prefix}{wrapper_method_name}() {{\n{source}\n}}\n}}"
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

        wrapped_program.functions.retain(|function| {
            if function.name == wrapper_function.name {
                return false;
            }
            if function.name.starts_with("__ayy_class_ctor_")
                && Self::eval_internal_function_name_hint(&function.name)
                    == Some(class_binding_name.as_str())
            {
                return false;
            }
            if function.name.starts_with("__ayy_class_init_")
                && function.body.iter().any(|statement| {
                    matches!(
                        statement,
                        Statement::Return(Expression::Identifier(name))
                            if name == &class_binding_name
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

    fn parse_eval_program_in_function_context(
        &self,
        source: &str,
        function_name: Option<&str>,
    ) -> Option<Program> {
        if frontend::script_goal_has_direct_using_declaration(source) {
            return None;
        }

        let Some(function_name) = function_name else {
            return self.parse_eval_program_in_current_function_context(source);
        };

        if self
            .resolve_registered_function_declaration(function_name)
            .is_some_and(|function| function.direct_eval_in_class_field_initializer)
            && let Some(program) = self
                .parse_eval_program_in_class_field_initializer_context_for_function(
                    function_name,
                    source,
                )
        {
            return Some(program);
        }

        if function_name.starts_with("__ayy_class_method_")
            && let Some(program) =
                self.parse_eval_program_in_class_method_context_for_function(function_name, source)
        {
            return Some(program);
        }

        if self
            .resolve_home_object_name_for_function(function_name)
            .is_some()
            && source.contains("super")
            && let Some(program) = self.parse_eval_program_in_method_context(source)
        {
            return Some(program);
        }

        self.parse_eval_program_in_ordinary_function_context(source)
    }

    fn parse_validated_static_direct_eval_program_with_context(
        &self,
        arguments: &[CallArgument],
        eval_function_name: Option<&str>,
    ) -> Result<Option<Program>, StaticThrowValue> {
        let Some(argument_source) = self.static_eval_argument_source_from_arguments(arguments)
        else {
            return Ok(None);
        };

        let raw_source = argument_source.clone();
        let eval_context_strict = eval_function_name
            .and_then(|function_name| self.resolve_registered_function_declaration(function_name))
            .is_some_and(|function| function.strict)
            || (eval_function_name.is_none()
                && self.state.speculation.execution_context.strict_mode);
        let argument_source = if eval_context_strict {
            let mut strict_argument_source = String::from("\"use strict\";");
            strict_argument_source.push_str(&argument_source);
            Cow::Owned(strict_argument_source)
        } else {
            Cow::Borrowed(argument_source.as_str())
        };

        let program = if let Some(program) =
            self.parse_eval_program_in_function_context(&argument_source, eval_function_name)
        {
            program
        } else if let Ok(program) = frontend::parse_script_goal(&argument_source) {
            program
        } else {
            return Err(StaticThrowValue::NamedError("SyntaxError"));
        };
        let mut program = lower_eval_static_function_constructors(program);

        namespace_eval_program_internal_function_names(
            &mut program,
            eval_function_name.or_else(|| self.current_function_name()),
            &raw_source,
        );
        self.normalize_eval_scoped_bindings_to_source_names(&mut program);

        if self.eval_arguments_initializer_conflict(&program)
            || self.eval_arguments_declaration_conflicts(&program)
            || self.eval_parameter_var_declaration_conflicts(&program)
            || self.eval_program_declares_var_collision_with_global_lexical(&program)
            || self.eval_program_declares_var_collision_with_active_lexical(&program)
        {
            return Err(StaticThrowValue::NamedError("SyntaxError"));
        }

        if self.eval_program_declares_non_definable_global_function(&program) {
            return Err(StaticThrowValue::NamedError("TypeError"));
        }

        if self.eval_program_declares_non_declarable_global_var(&program, false) {
            return Err(StaticThrowValue::NamedError("TypeError"));
        }

        Ok(Some(program))
    }

    fn parse_validated_static_direct_eval_program(
        &self,
        arguments: &[CallArgument],
    ) -> Result<Option<Program>, StaticThrowValue> {
        self.parse_validated_static_direct_eval_program_with_context(arguments, None)
    }

    fn static_eval_argument_source_from_expression(
        &self,
        expression: &Expression,
        visited: &mut HashSet<String>,
    ) -> Option<String> {
        if let Expression::String(source) = expression {
            return Some(source.clone());
        }
        if let Expression::Binary {
            op: BinaryOp::Add,
            left,
            right,
        } = expression
        {
            return Some(format!(
                "{}{}",
                self.static_eval_argument_source_from_expression(left, visited)?,
                self.static_eval_argument_source_from_expression(right, visited)?
            ));
        }

        if let Expression::Identifier(name) = expression {
            if !visited.insert(name.clone()) {
                return None;
            }

            let resolved_local_name = self
                .resolve_current_local_binding(name)
                .map(|(resolved_name, _)| resolved_name);
            if let Some(resolved_name) = resolved_local_name.as_deref() {
                if let Some(source) = self
                    .state
                    .speculation
                    .static_semantics
                    .local_value_binding(resolved_name)
                    .cloned()
                    .and_then(|value| {
                        self.static_eval_argument_source_from_expression(&value, visited)
                    })
                {
                    return Some(source);
                }
            }

            if let Some(source) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
                .cloned()
                .and_then(|value| self.static_eval_argument_source_from_expression(&value, visited))
            {
                return Some(source);
            }

            let capture_hidden_name = self.resolve_user_function_capture_hidden_name(name);
            if let Some(hidden_name) = capture_hidden_name.as_deref()
                && let Some(source) =
                    self.global_value_binding(hidden_name)
                        .cloned()
                        .and_then(|value| {
                            self.static_eval_argument_source_from_expression(&value, visited)
                        })
            {
                return Some(source);
            }
            if let Some(function_name) = self.current_function_name()
                && capture_hidden_name.is_some()
                && let Some(source) = self
                    .resolve_captured_alias_expression(function_name, name, &mut HashSet::new())
                    .and_then(|value| {
                        self.static_eval_argument_source_from_expression(&value, visited)
                    })
            {
                return Some(source);
            }
            if (resolved_local_name.is_none() || capture_hidden_name.is_some())
                && let Some(source) = self.global_value_binding(name).cloned().and_then(|value| {
                    self.static_eval_argument_source_from_expression(&value, visited)
                })
            {
                return Some(source);
            }
        }

        if let Some(resolved) = self.resolve_bound_alias_expression(expression)
            && !static_expression_matches(&resolved, expression)
            && let Some(source) =
                self.static_eval_argument_source_from_expression(&resolved, visited)
        {
            return Some(source);
        }

        let materialized = self.materialize_static_expression(expression);
        if !static_expression_matches(&materialized, expression) {
            return self.static_eval_argument_source_from_expression(&materialized, visited);
        }

        None
    }

    pub(in crate::backend::direct_wasm) fn static_eval_argument_source_from_arguments(
        &self,
        arguments: &[CallArgument],
    ) -> Option<String> {
        let argument = arguments.first()?;
        match argument {
            CallArgument::Expression(Expression::String(argument_source)) => {
                Some(argument_source.clone())
            }
            CallArgument::Expression(expression) => {
                self.static_eval_argument_source_from_expression(expression, &mut HashSet::new())
            }
            CallArgument::Spread(_) => None,
        }
    }

    fn parse_validated_static_indirect_eval_program_with_context(
        &self,
        arguments: &[CallArgument],
        current_function_name: Option<&str>,
    ) -> Result<Option<Program>, StaticThrowValue> {
        let Some(argument_source) = self.static_eval_argument_source_from_arguments(arguments)
        else {
            return Ok(None);
        };
        let mut program = frontend::parse_script_goal(&argument_source)
            .map_err(|_| StaticThrowValue::NamedError("SyntaxError"))?;
        program = lower_eval_static_function_constructors(program);
        namespace_eval_program_internal_function_names(
            &mut program,
            current_function_name,
            &argument_source,
        );
        if self.eval_program_declares_non_definable_global_function(&program) {
            return Err(StaticThrowValue::NamedError("TypeError"));
        }
        if self.eval_program_declares_non_declarable_global_var(&program, false) {
            return Err(StaticThrowValue::NamedError("TypeError"));
        }
        Ok(Some(program))
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_direct_eval_outcome(
        &self,
        arguments: &[CallArgument],
    ) -> Option<StaticEvalOutcome> {
        match self.parse_validated_static_direct_eval_program(arguments) {
            Ok(_) => None,
            Err(error) => Some(StaticEvalOutcome::Throw(error)),
        }
    }

    fn resolve_static_eval_statement_completion_expression(
        &self,
        statement: &Statement,
    ) -> Option<Expression> {
        match statement {
            Statement::Expression(expression) => Some(expression.clone()),
            Statement::Assign { value, .. } | Statement::AssignMember { value, .. } => {
                Some(value.clone())
            }
            Statement::Block { body } | Statement::Declaration { body } => {
                self.resolve_static_eval_statement_list_completion_expression(body)
            }
            Statement::With { body, .. } => self
                .resolve_static_eval_statement_list_completion_expression(body)
                .or(Some(Expression::Undefined)),
            Statement::Labeled { body, .. }
            | Statement::DoWhile { body, .. }
            | Statement::While { body, .. }
            | Statement::For { body, .. } => {
                self.resolve_static_eval_statement_list_completion_expression(body)
            }
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => {
                let then_completion =
                    self.resolve_static_eval_statement_list_completion_expression(then_branch);
                let else_completion =
                    self.resolve_static_eval_statement_list_completion_expression(else_branch);
                match (then_completion, else_completion) {
                    (Some(then_completion), Some(else_completion))
                        if static_expression_matches(&then_completion, &else_completion) =>
                    {
                        Some(then_completion)
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn resolve_static_eval_statement_list_completion_expression(
        &self,
        statements: &[Statement],
    ) -> Option<Expression> {
        let mut completion = None;
        for statement in statements {
            if let Some(statement_completion) =
                self.resolve_static_eval_statement_completion_expression(statement)
            {
                completion = Some(statement_completion);
            }
        }
        Some(completion.unwrap_or(Expression::Undefined))
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_direct_eval_completion_outcome_with_context(
        &self,
        arguments: &[CallArgument],
        eval_function_name: Option<&str>,
    ) -> Option<StaticEvalOutcome> {
        match self
            .parse_validated_static_direct_eval_program_with_context(arguments, eval_function_name)
        {
            Ok(Some(program)) => self
                .resolve_static_eval_statement_list_completion_expression(&program.statements)
                .map(StaticEvalOutcome::Value),
            Ok(None) => None,
            Err(error) => Some(StaticEvalOutcome::Throw(error)),
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_indirect_eval_completion_outcome_with_context(
        &self,
        arguments: &[CallArgument],
        current_function_name: Option<&str>,
    ) -> Option<StaticEvalOutcome> {
        match self.parse_validated_static_indirect_eval_program_with_context(
            arguments,
            current_function_name,
        ) {
            Ok(Some(program)) => self
                .resolve_static_eval_statement_list_completion_expression(&program.statements)
                .map(StaticEvalOutcome::Value),
            Ok(None) => None,
            Err(error) => Some(StaticEvalOutcome::Throw(error)),
        }
    }

    fn static_eval_statement_inline_effects(
        &self,
        statement: &Statement,
    ) -> Option<Vec<InlineFunctionEffect>> {
        match statement {
            Statement::Assign { name, value } => Some(vec![InlineFunctionEffect::Assign {
                name: name.clone(),
                value: value.clone(),
            }]),
            Statement::Expression(Expression::Assign { name, value }) => {
                Some(vec![InlineFunctionEffect::Assign {
                    name: name.clone(),
                    value: value.as_ref().clone(),
                }])
            }
            Statement::Expression(Expression::Update { name, op, prefix }) => {
                Some(vec![InlineFunctionEffect::Update {
                    name: name.clone(),
                    op: *op,
                    prefix: *prefix,
                }])
            }
            Statement::Expression(expression) => {
                Some(vec![InlineFunctionEffect::Expression(expression.clone())])
            }
            Statement::Block { body } | Statement::Declaration { body } => {
                self.static_eval_statement_list_inline_effects(body)
            }
            _ => None,
        }
    }

    fn static_eval_statement_list_inline_effects(
        &self,
        statements: &[Statement],
    ) -> Option<Vec<InlineFunctionEffect>> {
        let mut effects = Vec::new();
        for statement in statements {
            effects.extend(self.static_eval_statement_inline_effects(statement)?);
        }
        Some(effects)
    }

    pub(in crate::backend::direct_wasm) fn static_direct_eval_inline_effects_with_context(
        &self,
        arguments: &[CallArgument],
        eval_function_name: Option<&str>,
    ) -> Option<Vec<InlineFunctionEffect>> {
        let program = match self
            .parse_validated_static_direct_eval_program_with_context(arguments, eval_function_name)
        {
            Ok(Some(program)) => program,
            Ok(None) => return Some(Vec::new()),
            Err(_) => return None,
        };
        self.static_eval_statement_list_inline_effects(&program.statements)
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_direct_eval_return_outcome_from_user_function(
        &self,
        user_function: &UserFunction,
        function: &FunctionDeclaration,
        arguments: &[CallArgument],
        this_binding: &Expression,
    ) -> Option<StaticEvalOutcome> {
        let [Statement::Return(expression)] = function.body.as_slice() else {
            return None;
        };
        let arguments_binding = Expression::Array(
            arguments
                .iter()
                .map(|argument| match argument {
                    CallArgument::Expression(expression) => {
                        ArrayElement::Expression(expression.clone())
                    }
                    CallArgument::Spread(expression) => ArrayElement::Spread(expression.clone()),
                })
                .collect(),
        );
        let expression = self.substitute_user_function_call_frame_bindings(
            expression,
            user_function,
            arguments,
            this_binding,
            &arguments_binding,
        );
        let Expression::Call {
            callee,
            arguments: eval_arguments,
        } = expression
        else {
            return None;
        };
        if !matches!(callee.as_ref(), Expression::Identifier(name) if name == "eval") {
            return None;
        }
        self.resolve_static_direct_eval_completion_outcome_with_context(
            &eval_arguments,
            Some(&function.name),
        )
    }

    pub(in crate::backend::direct_wasm) fn infer_static_direct_eval_completion_kind(
        &self,
        arguments: &[CallArgument],
    ) -> Option<StaticValueKind> {
        let program = self
            .parse_validated_static_direct_eval_program(arguments)
            .ok()
            .flatten()?;
        self.infer_eval_statement_list_completion_kind(&program.statements)
    }

    fn infer_eval_statement_list_completion_kind(
        &self,
        statements: &[Statement],
    ) -> Option<StaticValueKind> {
        let mut completion_kind = None;
        for statement in statements {
            if let Some(kind) = self.infer_eval_statement_completion_kind(statement) {
                completion_kind = Some(kind);
            }
        }
        completion_kind
    }

    fn infer_eval_statement_completion_kind(
        &self,
        statement: &Statement,
    ) -> Option<StaticValueKind> {
        match statement {
            Statement::Expression(expression) => self.infer_value_kind(expression),
            Statement::Assign { value, .. } | Statement::AssignMember { value, .. } => {
                self.infer_value_kind(value)
            }
            Statement::Block { body } | Statement::Declaration { body } => {
                self.infer_eval_statement_list_completion_kind(body)
            }
            Statement::With { body, .. } => self
                .infer_eval_statement_list_completion_kind(body)
                .or(Some(StaticValueKind::Undefined)),
            Statement::DoWhile { body, .. }
            | Statement::While { body, .. }
            | Statement::For { body, .. } => self.infer_eval_statement_list_completion_kind(body),
            Statement::Labeled { body, .. } => self.infer_eval_statement_list_completion_kind(body),
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => {
                let then_kind = self.infer_eval_statement_list_completion_kind(then_branch);
                let else_kind = self.infer_eval_statement_list_completion_kind(else_branch);
                if then_kind == else_kind {
                    then_kind
                } else {
                    Some(StaticValueKind::Unknown)
                }
            }
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn eval_arguments_declaration_conflicts(
        &self,
        program: &Program,
    ) -> bool {
        if !eval_program_declares_var_arguments(program) {
            return false;
        }

        let Some(current_function) = self.current_user_function() else {
            return false;
        };

        self.state.parameters.in_parameter_default_initialization
            && (!current_function.lexical_this
                || current_function
                    .params
                    .iter()
                    .any(|parameter| parameter == "arguments"))
    }

    pub(in crate::backend::direct_wasm) fn eval_arguments_initializer_conflict(
        &self,
        program: &Program,
    ) -> bool {
        self.state
            .speculation
            .execution_context
            .direct_eval_in_class_field_initializer
            && eval_program_contains_arguments(program)
    }

    pub(in crate::backend::direct_wasm) fn eval_parameter_var_declaration_conflicts(
        &self,
        program: &Program,
    ) -> bool {
        if program.strict || !self.state.parameters.in_parameter_default_initialization {
            return false;
        }

        collect_eval_var_names(program).into_iter().any(|var_name| {
            self.state
                .parameters
                .parameter_names
                .iter()
                .any(|param_name| {
                    scoped_binding_source_name(param_name).unwrap_or(param_name.as_str())
                        == var_name
                })
        })
    }

    pub(in crate::backend::direct_wasm) fn eval_program_declares_var_collision_with_global_lexical(
        &self,
        program: &Program,
    ) -> bool {
        if !self.state.speculation.execution_context.top_level_function || program.strict {
            return false;
        }

        collect_eval_var_names(program)
            .into_iter()
            .any(|name| self.backend.global_has_lexical_binding(&name))
    }

    pub(in crate::backend::direct_wasm) fn eval_program_declares_var_collision_with_active_lexical(
        &self,
        program: &Program,
    ) -> bool {
        if program.strict {
            return false;
        }

        collect_eval_var_names(program).into_iter().any(|name| {
            self.state
                .emission
                .lexical_scopes
                .active_eval_lexical_binding_counts
                .contains_key(&name)
        })
    }

    pub(in crate::backend::direct_wasm) fn eval_program_declares_non_definable_global_function(
        &self,
        program: &Program,
    ) -> bool {
        if !self.state.speculation.execution_context.top_level_function {
            return false;
        }

        self.eval_program_declares_non_declarable_global_function(program)
    }

    pub(in crate::backend::direct_wasm) fn eval_program_declares_non_declarable_global_function(
        &self,
        program: &Program,
    ) -> bool {
        let mut declared_function_names = HashSet::new();
        program
            .functions
            .iter()
            .rev()
            .filter(|function| function.register_global)
            .filter(|function| declared_function_names.insert(function.name.as_str()))
            .any(|function| {
                is_non_definable_global_name(&function.name)
                    || !self.can_declare_global_function(&function.name)
            })
    }

    pub(in crate::backend::direct_wasm) fn eval_program_declares_non_declarable_global_var(
        &self,
        program: &Program,
        script_global_declarations: bool,
    ) -> bool {
        if !self.state.speculation.execution_context.top_level_function {
            return false;
        }
        if program.strict && !script_global_declarations {
            return false;
        }

        let declared_function_names = program
            .functions
            .iter()
            .filter(|function| function.register_global)
            .map(|function| function.name.as_str())
            .collect::<HashSet<_>>();

        collect_eval_var_names(program)
            .into_iter()
            .filter(|name| !declared_function_names.contains(name.as_str()))
            .any(|name| !self.can_declare_global_var(&name))
    }

    fn can_declare_global_var(&self, name: &str) -> bool {
        if self.backend.global_property_descriptor(name).is_some()
            || self.backend.global_binding_index(name).is_some()
        {
            return true;
        }

        self.global_object_extensible_for_declaration()
    }

    fn can_declare_global_function(&self, name: &str) -> bool {
        if let Some(descriptor) = self.backend.global_property_descriptor(name) {
            if descriptor.configurable {
                return true;
            }
            return descriptor.writable == Some(true)
                && descriptor.enumerable
                && !descriptor.has_get
                && !descriptor.has_set;
        }

        self.global_object_extensible_for_declaration()
    }

    fn global_object_extensible_for_declaration(&self) -> bool {
        let this_extensible = self
            .backend
            .global_object_binding("this")
            .map(|binding| binding.extensible);
        let global_this_extensible = self
            .backend
            .global_object_binding("globalThis")
            .map(|binding| binding.extensible);

        match (this_extensible, global_this_extensible) {
            (Some(false), _) | (_, Some(false)) => false,
            (Some(true), _) | (_, Some(true)) => true,
            _ => self
                .resolve_static_object_extensibility(&Expression::This)
                .unwrap_or(true),
        }
    }

    pub(in crate::backend::direct_wasm) fn parse_eval_program_in_current_function_context(
        &self,
        source: &str,
    ) -> Option<Program> {
        if frontend::script_goal_has_direct_using_declaration(source) {
            return None;
        }

        if self
            .state
            .speculation
            .execution_context
            .direct_eval_in_class_field_initializer
            && let Some(program) =
                self.parse_eval_program_in_class_field_initializer_context(source)
        {
            return Some(program);
        }

        let current_function_name = self.current_function_name();
        if let Some(current_function_name) = current_function_name {
            if current_function_name.starts_with("__ayy_class_method_")
                && let Some(program) = self.parse_eval_program_in_class_method_context_for_function(
                    current_function_name,
                    source,
                )
            {
                return Some(program);
            }

            if self
                .resolve_home_object_name_for_function(current_function_name)
                .is_some()
                && source.contains("super")
            {
                if let Some(program) = self.parse_eval_program_in_method_context(source) {
                    return Some(program);
                }
            }
        }

        if current_function_name.is_some()
            || self
                .state
                .speculation
                .execution_context
                .direct_eval_in_class_field_initializer
        {
            return self.parse_eval_program_in_ordinary_function_context(source);
        }

        None
    }

    pub(in crate::backend::direct_wasm) fn parse_eval_program_in_ordinary_function_context(
        &self,
        source: &str,
    ) -> Option<Program> {
        let wrapper_name = "__ayy_eval_new_target_context__";
        let wrapped_source = format!("function {wrapper_name}() {{\n{source}\n}}");
        let mut wrapped_program = match frontend::parse(&wrapped_source) {
            Ok(program) => program,
            Err(_) => return None,
        };
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

    pub(in crate::backend::direct_wasm) fn parse_eval_program_in_method_context(
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

pub(in crate::backend::direct_wasm) fn eval_program_contains_arguments(program: &Program) -> bool {
    program
        .statements
        .iter()
        .any(eval_statement_contains_arguments)
        || program
            .functions
            .iter()
            .any(|function| function.body.iter().any(eval_statement_contains_arguments))
}

fn eval_statement_contains_arguments(statement: &Statement) -> bool {
    match statement {
        Statement::Declaration { body }
        | Statement::Block { body }
        | Statement::Labeled { body, .. } => body.iter().any(eval_statement_contains_arguments),
        Statement::With { object, body } => {
            eval_expression_contains_arguments(object)
                || body.iter().any(eval_statement_contains_arguments)
        }
        Statement::Var { value, .. }
        | Statement::Let { value, .. }
        | Statement::Assign { value, .. }
        | Statement::Return(value)
        | Statement::Throw(value)
        | Statement::Expression(value) => eval_expression_contains_arguments(value),
        Statement::AssignMember {
            object,
            property,
            value,
        } => {
            eval_expression_contains_arguments(object)
                || eval_expression_contains_arguments(property)
                || eval_expression_contains_arguments(value)
        }
        Statement::Print { values } => values.iter().any(eval_expression_contains_arguments),
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            eval_expression_contains_arguments(condition)
                || then_branch.iter().any(eval_statement_contains_arguments)
                || else_branch.iter().any(eval_statement_contains_arguments)
        }
        Statement::While {
            condition, body, ..
        }
        | Statement::DoWhile {
            condition, body, ..
        } => {
            eval_expression_contains_arguments(condition)
                || body.iter().any(eval_statement_contains_arguments)
        }
        Statement::For {
            init,
            condition,
            update,
            body,
            ..
        } => {
            init.iter().any(eval_statement_contains_arguments)
                || condition
                    .as_ref()
                    .is_some_and(eval_expression_contains_arguments)
                || update
                    .as_ref()
                    .is_some_and(eval_expression_contains_arguments)
                || body.iter().any(eval_statement_contains_arguments)
        }
        Statement::Break { .. } | Statement::Continue { .. } => false,
        Statement::Try {
            body,
            catch_binding: _,
            catch_setup,
            catch_body,
        } => {
            body.iter().any(eval_statement_contains_arguments)
                || catch_setup.iter().any(eval_statement_contains_arguments)
                || catch_body.iter().any(eval_statement_contains_arguments)
        }
        Statement::Switch {
            discriminant,
            cases,
            ..
        } => {
            eval_expression_contains_arguments(discriminant)
                || cases.iter().any(|case| {
                    case.test
                        .as_ref()
                        .is_some_and(eval_expression_contains_arguments)
                        || case.body.iter().any(eval_statement_contains_arguments)
                })
        }
        Statement::Yield { value } | Statement::YieldDelegate { value } => {
            eval_expression_contains_arguments(value)
        }
    }
}

fn eval_expression_contains_arguments(expression: &Expression) -> bool {
    match expression {
        Expression::Identifier(name) => name == "arguments",
        Expression::Null
        | Expression::Undefined
        | Expression::Bool(_)
        | Expression::Number(_)
        | Expression::BigInt(_)
        | Expression::String(_)
        | Expression::NewTarget
        | Expression::This
        | Expression::Sent => false,
        Expression::Array(elements) => elements.iter().any(|element| match element {
            ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                eval_expression_contains_arguments(expression)
            }
        }),
        Expression::Object(entries) => entries.iter().any(|entry| match entry {
            ObjectEntry::Data { key, value } => {
                eval_expression_contains_arguments(key) || eval_expression_contains_arguments(value)
            }
            ObjectEntry::Getter { key, getter }
            | ObjectEntry::Setter {
                key,
                setter: getter,
            } => {
                eval_expression_contains_arguments(key)
                    || eval_expression_contains_arguments(getter)
            }
            ObjectEntry::Spread(expression) => eval_expression_contains_arguments(expression),
        }),
        Expression::Member { object, property } => {
            eval_expression_contains_arguments(object)
                || eval_expression_contains_arguments(property)
        }
        Expression::SuperMember { property }
        | Expression::Await(property)
        | Expression::EnumerateKeys(property)
        | Expression::GetIterator(property)
        | Expression::IteratorClose(property) => eval_expression_contains_arguments(property),
        Expression::Unary { expression, .. } => eval_expression_contains_arguments(expression),
        Expression::Binary { left, right, .. } => {
            eval_expression_contains_arguments(left) || eval_expression_contains_arguments(right)
        }
        Expression::Conditional {
            condition,
            then_expression,
            else_expression,
        } => {
            eval_expression_contains_arguments(condition)
                || eval_expression_contains_arguments(then_expression)
                || eval_expression_contains_arguments(else_expression)
        }
        Expression::Assign { value, .. } => eval_expression_contains_arguments(value),
        Expression::AssignMember {
            object,
            property,
            value,
        } => {
            eval_expression_contains_arguments(object)
                || eval_expression_contains_arguments(property)
                || eval_expression_contains_arguments(value)
        }
        Expression::AssignSuperMember { property, value } => {
            eval_expression_contains_arguments(property)
                || eval_expression_contains_arguments(value)
        }
        Expression::Call { callee, arguments }
        | Expression::New { callee, arguments }
        | Expression::SuperCall { callee, arguments } => {
            eval_expression_contains_arguments(callee)
                || arguments.iter().any(|argument| match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        eval_expression_contains_arguments(expression)
                    }
                })
        }
        Expression::Sequence(expressions) => {
            expressions.iter().any(eval_expression_contains_arguments)
        }
        Expression::Update { .. } => false,
    }
}
