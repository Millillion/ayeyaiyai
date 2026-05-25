use super::*;

impl DirectWasmCompiler {
    pub(in crate::backend::direct_wasm) fn register_template_object_sites(
        &mut self,
        program: &Program,
    ) {
        self.state.template_object_array_bindings.clear();
        self.state.template_object_raw_array_bindings.clear();
        for function in &program.functions {
            for parameter in &function.params {
                if let Some(default) = &parameter.default {
                    self.register_template_object_sites_in_expression(default);
                }
            }
            self.register_template_object_sites_in_statements(&function.body);
        }
        self.register_template_object_sites_in_statements(&program.statements);
    }

    fn template_object_runtime_value_from_site_key(site_key: &str) -> Option<i32> {
        let site_id = site_key
            .strip_prefix("template-site:")?
            .parse::<i32>()
            .ok()?;
        site_id
            .checked_add(1)
            .and_then(|offset| JS_TEMPLATE_OBJECT_VALUE_BASE.checked_sub(offset))
    }

    pub(in crate::backend::direct_wasm) fn template_object_array_binding_from_array(
        value: &Expression,
    ) -> Option<ArrayValueBinding> {
        let Expression::Array(elements) = value else {
            return None;
        };
        Some(ArrayValueBinding {
            values: elements
                .iter()
                .map(|element| match element {
                    ArrayElement::Expression(value) => Some(value.clone()),
                    ArrayElement::Spread(_) => None,
                })
                .collect(),
        })
    }

    fn register_template_object_site_call(
        &mut self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) {
        if !matches!(callee, Expression::Identifier(name) if name == "__ayyTemplateObject") {
            return;
        }
        let Some(
            CallArgument::Expression(Expression::String(site_key))
            | CallArgument::Spread(Expression::String(site_key)),
        ) = arguments.first()
        else {
            return;
        };
        let Some(runtime_value) = Self::template_object_runtime_value_from_site_key(site_key)
        else {
            return;
        };
        let Some(CallArgument::Expression(cooked) | CallArgument::Spread(cooked)) =
            arguments.get(1)
        else {
            return;
        };
        let raw = arguments.get(2).and_then(|argument| match argument {
            CallArgument::Expression(raw) | CallArgument::Spread(raw) => {
                Self::template_object_array_binding_from_array(raw)
            }
        });
        if let Some(binding) = Self::template_object_array_binding_from_array(cooked)
            .or_else(|| self.infer_global_array_binding(cooked))
        {
            if std::env::var_os("AYY_TRACE_TEMPLATE_OBJECTS").is_some() {
                eprintln!(
                    "template_register site={site_key} runtime={runtime_value} len={}",
                    binding.values.len()
                );
            }
            self.state
                .template_object_array_bindings
                .insert(runtime_value, binding);
            if let Some(raw_binding) = raw {
                self.state
                    .template_object_raw_array_bindings
                    .insert(runtime_value, raw_binding);
            }
        } else if std::env::var_os("AYY_TRACE_TEMPLATE_OBJECTS").is_some() {
            eprintln!("template_register:missing_binding site={site_key} cooked={cooked:?}");
        }
    }

    fn register_template_object_sites_in_statements(&mut self, statements: &[Statement]) {
        for statement in statements {
            self.register_template_object_sites_in_statement(statement);
        }
    }

    fn register_template_object_sites_in_statement(&mut self, statement: &Statement) {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => {
                self.register_template_object_sites_in_statements(body);
            }
            Statement::Var { value, .. }
            | Statement::Let { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Expression(value)
            | Statement::Throw(value)
            | Statement::Return(value)
            | Statement::Yield { value }
            | Statement::YieldDelegate { value } => {
                self.register_template_object_sites_in_expression(value);
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                self.register_template_object_sites_in_expression(object);
                self.register_template_object_sites_in_expression(property);
                self.register_template_object_sites_in_expression(value);
            }
            Statement::Print { values } => {
                for value in values {
                    self.register_template_object_sites_in_expression(value);
                }
            }
            Statement::With { object, body } => {
                self.register_template_object_sites_in_expression(object);
                self.register_template_object_sites_in_statements(body);
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.register_template_object_sites_in_expression(condition);
                self.register_template_object_sites_in_statements(then_branch);
                self.register_template_object_sites_in_statements(else_branch);
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                self.register_template_object_sites_in_statements(body);
                self.register_template_object_sites_in_statements(catch_setup);
                self.register_template_object_sites_in_statements(catch_body);
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                self.register_template_object_sites_in_expression(discriminant);
                for case in cases {
                    if let Some(test) = &case.test {
                        self.register_template_object_sites_in_expression(test);
                    }
                    self.register_template_object_sites_in_statements(&case.body);
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
                self.register_template_object_sites_in_statements(init);
                if let Some(condition) = condition {
                    self.register_template_object_sites_in_expression(condition);
                }
                if let Some(update) = update {
                    self.register_template_object_sites_in_expression(update);
                }
                if let Some(break_hook) = break_hook {
                    self.register_template_object_sites_in_expression(break_hook);
                }
                self.register_template_object_sites_in_statements(body);
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
                self.register_template_object_sites_in_expression(condition);
                if let Some(break_hook) = break_hook {
                    self.register_template_object_sites_in_expression(break_hook);
                }
                self.register_template_object_sites_in_statements(body);
            }
            Statement::Break { .. } | Statement::Continue { .. } => {}
        }
    }

    fn register_template_object_sites_in_expression(&mut self, expression: &Expression) {
        match expression {
            Expression::Array(elements) => {
                for element in elements {
                    match element {
                        ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                            self.register_template_object_sites_in_expression(expression);
                        }
                    }
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            self.register_template_object_sites_in_expression(key);
                            self.register_template_object_sites_in_expression(value);
                        }
                        ObjectEntry::Getter { key, getter } => {
                            self.register_template_object_sites_in_expression(key);
                            self.register_template_object_sites_in_expression(getter);
                        }
                        ObjectEntry::Setter { key, setter } => {
                            self.register_template_object_sites_in_expression(key);
                            self.register_template_object_sites_in_expression(setter);
                        }
                        ObjectEntry::Spread(expression) => {
                            self.register_template_object_sites_in_expression(expression);
                        }
                    }
                }
            }
            Expression::Member { object, property }
            | Expression::AssignMember {
                object, property, ..
            } => {
                self.register_template_object_sites_in_expression(object);
                self.register_template_object_sites_in_expression(property);
                if let Expression::AssignMember { value, .. } = expression {
                    self.register_template_object_sites_in_expression(value);
                }
            }
            Expression::SuperMember { property }
            | Expression::AssignSuperMember { property, .. } => {
                self.register_template_object_sites_in_expression(property);
                if let Expression::AssignSuperMember { value, .. } = expression {
                    self.register_template_object_sites_in_expression(value);
                }
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => {
                self.register_template_object_sites_in_expression(value);
            }
            Expression::Binary { left, right, .. } => {
                self.register_template_object_sites_in_expression(left);
                self.register_template_object_sites_in_expression(right);
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.register_template_object_sites_in_expression(condition);
                self.register_template_object_sites_in_expression(then_expression);
                self.register_template_object_sites_in_expression(else_expression);
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    self.register_template_object_sites_in_expression(expression);
                }
            }
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                self.register_template_object_site_call(callee, arguments);
                self.register_template_object_sites_in_expression(callee);
                for argument in arguments {
                    self.register_template_object_sites_in_expression(argument.expression());
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
}
