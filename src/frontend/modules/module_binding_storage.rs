use super::*;

pub(super) struct ModuleBindingStorageRewriter<'a> {
    storage_bindings: &'a BTreeMap<String, String>,
    scopes: Vec<HashSet<String>>,
}

impl<'a> ModuleBindingStorageRewriter<'a> {
    pub(super) fn new(storage_bindings: &'a BTreeMap<String, String>) -> Self {
        Self {
            storage_bindings,
            scopes: Vec::new(),
        }
    }

    pub(super) fn rewrite_module_statement_list(
        &mut self,
        statements: &mut [Statement],
    ) -> Result<()> {
        self.scopes.push(HashSet::new());
        for statement in statements {
            self.rewrite_statement(statement, true)?;
        }
        self.scopes.pop();
        Ok(())
    }

    pub(super) fn rewrite_function(&mut self, function: &mut FunctionDeclaration) -> Result<()> {
        let mut function_scope = function
            .params
            .iter()
            .map(|parameter| parameter.name.clone())
            .collect::<HashSet<_>>();
        if let Some(self_binding) = &function.self_binding {
            function_scope.insert(self_binding.clone());
        }
        function_scope.insert("arguments".to_string());
        self.scopes.push(function_scope);
        self.rewrite_statement_list(&mut function.body)?;
        self.scopes.pop();
        Ok(())
    }

    pub(super) fn rewrite_expression(&mut self, expression: &mut Expression) -> Result<()> {
        match expression {
            Expression::Array(elements) => {
                for element in elements {
                    match element {
                        ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                            self.rewrite_expression(expression)?
                        }
                    }
                }
                Ok(())
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            self.rewrite_expression(key)?;
                            self.rewrite_expression(value)?;
                        }
                        ObjectEntry::Getter { key, getter }
                        | ObjectEntry::Setter {
                            key,
                            setter: getter,
                        } => {
                            self.rewrite_expression(key)?;
                            self.rewrite_expression(getter)?;
                        }
                        ObjectEntry::Spread(expression) => self.rewrite_expression(expression)?,
                    }
                }
                Ok(())
            }
            Expression::Identifier(name) => {
                if let Some(storage_name) = self.storage_binding_for(name) {
                    *name = storage_name;
                }
                Ok(())
            }
            Expression::Member { object, property } => {
                self.rewrite_expression(object)?;
                self.rewrite_expression(property)
            }
            Expression::SuperMember { property } => self.rewrite_expression(property),
            Expression::Assign { name, value } => {
                self.rewrite_expression(value)?;
                if let Some(storage_name) = self.storage_binding_for(name) {
                    *name = storage_name;
                }
                Ok(())
            }
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.rewrite_expression(object)?;
                self.rewrite_expression(property)?;
                self.rewrite_expression(value)
            }
            Expression::AssignSuperMember { property, value } => {
                self.rewrite_expression(property)?;
                self.rewrite_expression(value)
            }
            Expression::Await(expression)
            | Expression::EnumerateKeys(expression)
            | Expression::GetIterator(expression)
            | Expression::IteratorClose(expression)
            | Expression::Unary { expression, .. } => self.rewrite_expression(expression),
            Expression::Binary { left, right, .. } => {
                self.rewrite_expression(left)?;
                self.rewrite_expression(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.rewrite_expression(condition)?;
                self.rewrite_expression(then_expression)?;
                self.rewrite_expression(else_expression)
            }
            Expression::Sequence(elements) => {
                for element in elements {
                    self.rewrite_expression(element)?;
                }
                Ok(())
            }
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                self.rewrite_expression(callee)?;
                for argument in arguments {
                    match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.rewrite_expression(expression)?
                        }
                    }
                }
                Ok(())
            }
            Expression::Update { name, .. } => {
                if let Some(storage_name) = self.storage_binding_for(name) {
                    *name = storage_name;
                }
                Ok(())
            }
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::NewTarget
            | Expression::This
            | Expression::Sent => Ok(()),
        }
    }

    fn rewrite_statement_list(&mut self, statements: &mut [Statement]) -> Result<()> {
        self.scopes.push(
            collect_statement_bindings(statements.iter())
                .into_iter()
                .collect(),
        );
        for statement in statements {
            self.rewrite_statement(statement, false)?;
        }
        self.scopes.pop();
        Ok(())
    }

    fn rewrite_statement(&mut self, statement: &mut Statement, module_root: bool) -> Result<()> {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => self.rewrite_statement_list(body),
            Statement::Var { name, value } | Statement::Let { name, value, .. } => {
                self.rewrite_expression(value)?;
                if module_root
                    && let Some(storage_name) = self.storage_binding_for(name)
                {
                    *statement = Statement::Assign {
                        name: storage_name,
                        value: value.clone(),
                    };
                }
                Ok(())
            }
            Statement::Assign { name, value } => {
                self.rewrite_expression(value)?;
                if let Some(storage_name) = self.storage_binding_for(name) {
                    *name = storage_name;
                }
                Ok(())
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                self.rewrite_expression(object)?;
                self.rewrite_expression(property)?;
                self.rewrite_expression(value)
            }
            Statement::Print { values } => {
                for value in values {
                    self.rewrite_expression(value)?;
                }
                Ok(())
            }
            Statement::Expression(expression)
            | Statement::Throw(expression)
            | Statement::Return(expression)
            | Statement::Yield { value: expression }
            | Statement::YieldDelegate { value: expression } => self.rewrite_expression(expression),
            Statement::With { object, body } => {
                self.rewrite_expression(object)?;
                self.rewrite_statement_list(body)
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.rewrite_expression(condition)?;
                self.rewrite_statement_list(then_branch)?;
                self.rewrite_statement_list(else_branch)
            }
            Statement::Try {
                body,
                catch_binding,
                catch_setup,
                catch_body,
            } => {
                self.rewrite_statement_list(body)?;
                let mut catch_scope =
                    collect_statement_bindings(catch_setup.iter().chain(catch_body.iter()))
                        .into_iter()
                        .collect::<HashSet<_>>();
                if let Some(catch_binding) = catch_binding {
                    catch_scope.insert(catch_binding.clone());
                }
                self.scopes.push(catch_scope);
                for statement in catch_setup {
                    self.rewrite_statement(statement, false)?;
                }
                for statement in catch_body {
                    self.rewrite_statement(statement, false)?;
                }
                self.scopes.pop();
                Ok(())
            }
            Statement::Switch {
                bindings,
                discriminant,
                cases,
                ..
            } => {
                self.rewrite_expression(discriminant)?;
                self.scopes.push(bindings.iter().cloned().collect());
                for case in cases {
                    if let Some(test) = &mut case.test {
                        self.rewrite_expression(test)?;
                    }
                    self.rewrite_statement_list(&mut case.body)?;
                }
                self.scopes.pop();
                Ok(())
            }
            Statement::For {
                init,
                per_iteration_bindings,
                condition,
                update,
                break_hook,
                body,
                ..
            } => {
                let mut loop_bindings = collect_statement_bindings(init.iter())
                    .into_iter()
                    .collect::<HashSet<_>>();
                loop_bindings.extend(per_iteration_bindings.iter().cloned());
                self.scopes.push(loop_bindings);
                for statement in init {
                    self.rewrite_statement(statement, false)?;
                }
                if let Some(condition) = condition {
                    self.rewrite_expression(condition)?;
                }
                if let Some(update) = update {
                    self.rewrite_expression(update)?;
                }
                if let Some(break_hook) = break_hook {
                    self.rewrite_expression(break_hook)?;
                }
                self.rewrite_statement_list(body)?;
                self.scopes.pop();
                Ok(())
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
                self.rewrite_expression(condition)?;
                if let Some(break_hook) = break_hook {
                    self.rewrite_expression(break_hook)?;
                }
                self.rewrite_statement_list(body)
            }
            Statement::Break { .. } | Statement::Continue { .. } => Ok(()),
        }
    }

    fn storage_binding_for(&self, name: &str) -> Option<String> {
        if self.is_shadowed(name) {
            None
        } else {
            self.storage_bindings.get(name).cloned()
        }
    }

    fn is_shadowed(&self, name: &str) -> bool {
        self.scopes.iter().rev().any(|scope| scope.contains(name))
    }
}

pub(super) fn collect_statement_bindings<'a>(
    statements: impl Iterator<Item = &'a Statement>,
) -> Vec<String> {
    let mut bindings = Vec::new();
    let mut seen = HashSet::new();
    for statement in statements {
        match statement {
            Statement::Var { name, .. } | Statement::Let { name, .. } => {
                if seen.insert(name.clone()) {
                    bindings.push(name.clone());
                }
            }
            _ => {}
        }
    }
    bindings
}
