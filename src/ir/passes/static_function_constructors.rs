use std::collections::{HashMap, HashSet};

use anyhow::{Result, bail};

use crate::ir::hir::{
    ArrayElement, CallArgument, Expression, FunctionDeclaration, ObjectEntry, Program, Statement,
    SwitchCase,
};

use super::{scope_stack::ScopeStack, support::collect_statement_bindings};

mod function_constructor;
mod realm_eval;

pub fn lower(program: Program) -> Result<Program> {
    StaticFunctionConstructorLowerer::new(&program).lower(program)
}

struct StaticFunctionConstructorLowerer {
    scopes: ScopeStack,
    global_scope: HashSet<String>,
    known_string_values: Vec<HashMap<String, String>>,
    known_test262_realms: Vec<HashSet<String>>,
    existing_function_names: HashSet<String>,
    synthetic_functions: Vec<FunctionDeclaration>,
    next_synthetic_function_id: usize,
    next_template_object_id: usize,
}

impl StaticFunctionConstructorLowerer {
    fn new(program: &Program) -> Self {
        let mut global_scope = collect_statement_bindings(program.statements.iter())
            .into_iter()
            .collect::<HashSet<_>>();
        global_scope.extend(
            program
                .functions
                .iter()
                .filter(|function| function.register_global)
                .map(|function| function.name.clone()),
        );

        Self {
            scopes: ScopeStack::default(),
            global_scope,
            known_string_values: Vec::new(),
            known_test262_realms: Vec::new(),
            existing_function_names: program
                .functions
                .iter()
                .map(|function| function.name.clone())
                .collect(),
            synthetic_functions: Vec::new(),
            next_synthetic_function_id: 0,
            next_template_object_id: Self::next_template_object_id_after_program(program),
        }
    }

    fn next_template_object_id_after_program(program: &Program) -> usize {
        let mut next_id = 0;
        Self::collect_next_template_object_id_from_statements(&program.statements, &mut next_id);
        for function in &program.functions {
            Self::collect_next_template_object_id_from_function(function, &mut next_id);
        }
        next_id
    }

    fn collect_next_template_object_id_from_function(
        function: &FunctionDeclaration,
        next_id: &mut usize,
    ) {
        for parameter in &function.params {
            if let Some(default) = &parameter.default {
                Self::collect_next_template_object_id_from_expression(default, next_id);
            }
        }
        Self::collect_next_template_object_id_from_statements(&function.body, next_id);
    }

    fn collect_next_template_object_id_from_statements(
        statements: &[Statement],
        next_id: &mut usize,
    ) {
        for statement in statements {
            Self::collect_next_template_object_id_from_statement(statement, next_id);
        }
    }

    fn collect_next_template_object_id_from_statement(statement: &Statement, next_id: &mut usize) {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. } => {
                Self::collect_next_template_object_id_from_statements(body, next_id);
            }
            Statement::Var { value, .. }
            | Statement::Let { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Throw(value)
            | Statement::Return(value)
            | Statement::Yield { value }
            | Statement::YieldDelegate { value }
            | Statement::Expression(value) => {
                Self::collect_next_template_object_id_from_expression(value, next_id);
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                Self::collect_next_template_object_id_from_expression(object, next_id);
                Self::collect_next_template_object_id_from_expression(property, next_id);
                Self::collect_next_template_object_id_from_expression(value, next_id);
            }
            Statement::Print { values } => {
                for value in values {
                    Self::collect_next_template_object_id_from_expression(value, next_id);
                }
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                Self::collect_next_template_object_id_from_expression(condition, next_id);
                Self::collect_next_template_object_id_from_statements(then_branch, next_id);
                Self::collect_next_template_object_id_from_statements(else_branch, next_id);
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                Self::collect_next_template_object_id_from_statements(body, next_id);
                Self::collect_next_template_object_id_from_statements(catch_setup, next_id);
                Self::collect_next_template_object_id_from_statements(catch_body, next_id);
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                Self::collect_next_template_object_id_from_expression(discriminant, next_id);
                for case in cases {
                    if let Some(test) = &case.test {
                        Self::collect_next_template_object_id_from_expression(test, next_id);
                    }
                    Self::collect_next_template_object_id_from_statements(&case.body, next_id);
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
                Self::collect_next_template_object_id_from_statements(init, next_id);
                if let Some(condition) = condition {
                    Self::collect_next_template_object_id_from_expression(condition, next_id);
                }
                if let Some(update) = update {
                    Self::collect_next_template_object_id_from_expression(update, next_id);
                }
                if let Some(break_hook) = break_hook {
                    Self::collect_next_template_object_id_from_expression(break_hook, next_id);
                }
                Self::collect_next_template_object_id_from_statements(body, next_id);
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
                Self::collect_next_template_object_id_from_expression(condition, next_id);
                if let Some(break_hook) = break_hook {
                    Self::collect_next_template_object_id_from_expression(break_hook, next_id);
                }
                Self::collect_next_template_object_id_from_statements(body, next_id);
            }
            Statement::Break { .. } | Statement::Continue { .. } => {}
        }
    }

    fn collect_next_template_object_id_from_expression(
        expression: &Expression,
        next_id: &mut usize,
    ) {
        if let Expression::Call { callee, arguments } = expression
            && matches!(callee.as_ref(), Expression::Identifier(name) if name == "__ayyTemplateObject")
            && let Some(
                CallArgument::Expression(Expression::String(site_key))
                | CallArgument::Spread(Expression::String(site_key)),
            ) = arguments.first()
            && let Some(site_id) = site_key
                .strip_prefix("template-site:")
                .and_then(|suffix| suffix.parse::<usize>().ok())
        {
            *next_id = (*next_id).max(site_id.saturating_add(1));
        }

        match expression {
            Expression::Array(elements) => {
                for element in elements {
                    match element {
                        ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                            Self::collect_next_template_object_id_from_expression(
                                expression, next_id,
                            );
                        }
                    }
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            Self::collect_next_template_object_id_from_expression(key, next_id);
                            Self::collect_next_template_object_id_from_expression(value, next_id);
                        }
                        ObjectEntry::Getter { key, getter } => {
                            Self::collect_next_template_object_id_from_expression(key, next_id);
                            Self::collect_next_template_object_id_from_expression(getter, next_id);
                        }
                        ObjectEntry::Setter { key, setter } => {
                            Self::collect_next_template_object_id_from_expression(key, next_id);
                            Self::collect_next_template_object_id_from_expression(setter, next_id);
                        }
                        ObjectEntry::Spread(expression) => {
                            Self::collect_next_template_object_id_from_expression(
                                expression, next_id,
                            );
                        }
                    }
                }
            }
            Expression::Member { object, property }
            | Expression::Binary {
                left: object,
                right: property,
                ..
            } => {
                Self::collect_next_template_object_id_from_expression(object, next_id);
                Self::collect_next_template_object_id_from_expression(property, next_id);
            }
            Expression::SuperMember { property } => {
                Self::collect_next_template_object_id_from_expression(property, next_id);
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => {
                Self::collect_next_template_object_id_from_expression(value, next_id);
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::collect_next_template_object_id_from_expression(condition, next_id);
                Self::collect_next_template_object_id_from_expression(then_expression, next_id);
                Self::collect_next_template_object_id_from_expression(else_expression, next_id);
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    Self::collect_next_template_object_id_from_expression(expression, next_id);
                }
            }
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                Self::collect_next_template_object_id_from_expression(callee, next_id);
                for argument in arguments {
                    Self::collect_next_template_object_id_from_expression(
                        argument.expression(),
                        next_id,
                    );
                }
            }
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                Self::collect_next_template_object_id_from_expression(object, next_id);
                Self::collect_next_template_object_id_from_expression(property, next_id);
                Self::collect_next_template_object_id_from_expression(value, next_id);
            }
            Expression::AssignSuperMember { property, value } => {
                Self::collect_next_template_object_id_from_expression(property, next_id);
                Self::collect_next_template_object_id_from_expression(value, next_id);
            }
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::Identifier(_)
            | Expression::Update { .. }
            | Expression::NewTarget
            | Expression::This
            | Expression::Sent => {}
        }
    }

    fn fresh_template_object_site_key(&mut self) -> String {
        let site_id = self.next_template_object_id;
        self.next_template_object_id += 1;
        format!("template-site:{site_id}")
    }

    fn renumber_template_object_sites_in_function(&mut self, function: &mut FunctionDeclaration) {
        for parameter in &mut function.params {
            if let Some(default) = &mut parameter.default {
                self.renumber_template_object_sites_in_expression(default);
            }
        }
        self.renumber_template_object_sites_in_statements(&mut function.body);
    }

    fn renumber_template_object_sites_in_statements(&mut self, statements: &mut [Statement]) {
        for statement in statements {
            self.renumber_template_object_sites_in_statement(statement);
        }
    }

    fn renumber_template_object_sites_in_statement(&mut self, statement: &mut Statement) {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. } => {
                self.renumber_template_object_sites_in_statements(body);
            }
            Statement::Var { value, .. }
            | Statement::Let { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Throw(value)
            | Statement::Return(value)
            | Statement::Yield { value }
            | Statement::YieldDelegate { value }
            | Statement::Expression(value) => {
                self.renumber_template_object_sites_in_expression(value);
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                self.renumber_template_object_sites_in_expression(object);
                self.renumber_template_object_sites_in_expression(property);
                self.renumber_template_object_sites_in_expression(value);
            }
            Statement::Print { values } => {
                for value in values {
                    self.renumber_template_object_sites_in_expression(value);
                }
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.renumber_template_object_sites_in_expression(condition);
                self.renumber_template_object_sites_in_statements(then_branch);
                self.renumber_template_object_sites_in_statements(else_branch);
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                self.renumber_template_object_sites_in_statements(body);
                self.renumber_template_object_sites_in_statements(catch_setup);
                self.renumber_template_object_sites_in_statements(catch_body);
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                self.renumber_template_object_sites_in_expression(discriminant);
                for case in cases {
                    if let Some(test) = &mut case.test {
                        self.renumber_template_object_sites_in_expression(test);
                    }
                    self.renumber_template_object_sites_in_statements(&mut case.body);
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
                self.renumber_template_object_sites_in_statements(init);
                if let Some(condition) = condition {
                    self.renumber_template_object_sites_in_expression(condition);
                }
                if let Some(update) = update {
                    self.renumber_template_object_sites_in_expression(update);
                }
                if let Some(break_hook) = break_hook {
                    self.renumber_template_object_sites_in_expression(break_hook);
                }
                self.renumber_template_object_sites_in_statements(body);
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
                self.renumber_template_object_sites_in_expression(condition);
                if let Some(break_hook) = break_hook {
                    self.renumber_template_object_sites_in_expression(break_hook);
                }
                self.renumber_template_object_sites_in_statements(body);
            }
            Statement::Break { .. } | Statement::Continue { .. } => {}
        }
    }

    fn renumber_template_object_sites_in_expression(&mut self, expression: &mut Expression) {
        if let Expression::Call { callee, arguments } = expression {
            let should_renumber = matches!(callee.as_ref(), Expression::Identifier(name) if name == "__ayyTemplateObject")
                && matches!(
                    arguments.first(),
                    Some(
                        CallArgument::Expression(Expression::String(site_key))
                            | CallArgument::Spread(Expression::String(site_key))
                    ) if site_key.starts_with("template-site:")
                );
            if should_renumber {
                let site_key = self.fresh_template_object_site_key();
                match arguments.first_mut() {
                    Some(
                        CallArgument::Expression(Expression::String(current))
                        | CallArgument::Spread(Expression::String(current)),
                    ) => {
                        *current = site_key;
                    }
                    _ => {}
                }
            }
        }

        match expression {
            Expression::Array(elements) => {
                for element in elements {
                    match element {
                        ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                            self.renumber_template_object_sites_in_expression(expression);
                        }
                    }
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            self.renumber_template_object_sites_in_expression(key);
                            self.renumber_template_object_sites_in_expression(value);
                        }
                        ObjectEntry::Getter { key, getter } => {
                            self.renumber_template_object_sites_in_expression(key);
                            self.renumber_template_object_sites_in_expression(getter);
                        }
                        ObjectEntry::Setter { key, setter } => {
                            self.renumber_template_object_sites_in_expression(key);
                            self.renumber_template_object_sites_in_expression(setter);
                        }
                        ObjectEntry::Spread(expression) => {
                            self.renumber_template_object_sites_in_expression(expression);
                        }
                    }
                }
            }
            Expression::Member { object, property }
            | Expression::Binary {
                left: object,
                right: property,
                ..
            } => {
                self.renumber_template_object_sites_in_expression(object);
                self.renumber_template_object_sites_in_expression(property);
            }
            Expression::SuperMember { property } => {
                self.renumber_template_object_sites_in_expression(property);
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => {
                self.renumber_template_object_sites_in_expression(value);
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.renumber_template_object_sites_in_expression(condition);
                self.renumber_template_object_sites_in_expression(then_expression);
                self.renumber_template_object_sites_in_expression(else_expression);
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    self.renumber_template_object_sites_in_expression(expression);
                }
            }
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                self.renumber_template_object_sites_in_expression(callee);
                for argument in arguments {
                    self.renumber_template_object_sites_in_expression(argument.expression_mut());
                }
            }
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.renumber_template_object_sites_in_expression(object);
                self.renumber_template_object_sites_in_expression(property);
                self.renumber_template_object_sites_in_expression(value);
            }
            Expression::AssignSuperMember { property, value } => {
                self.renumber_template_object_sites_in_expression(property);
                self.renumber_template_object_sites_in_expression(value);
            }
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::Identifier(_)
            | Expression::Update { .. }
            | Expression::NewTarget
            | Expression::This
            | Expression::Sent => {}
        }
    }

    fn lower(mut self, mut program: Program) -> Result<Program> {
        self.scopes.push(self.global_scope.clone());
        self.known_string_values.push(HashMap::new());
        self.known_test262_realms.push(HashSet::new());
        program.statements = self.lower_statement_list(program.statements)?;

        let original_functions = std::mem::take(&mut program.functions);
        let mut lowered_functions = Vec::with_capacity(original_functions.len());
        for function in original_functions {
            lowered_functions.push(self.lower_function(function)?);
        }
        self.known_test262_realms.pop();
        self.known_string_values.pop();
        self.scopes.pop();

        lowered_functions.extend(self.synthetic_functions);
        program.functions = lowered_functions;
        Ok(program)
    }

    fn lower_function(&mut self, mut function: FunctionDeclaration) -> Result<FunctionDeclaration> {
        let mut function_scope = collect_statement_bindings(function.body.iter())
            .into_iter()
            .collect::<HashSet<_>>();
        function_scope.extend(
            function
                .params
                .iter()
                .map(|parameter| parameter.name.clone()),
        );
        if let Some(self_binding) = &function.self_binding {
            function_scope.insert(self_binding.clone());
        }
        function_scope.insert("arguments".to_string());

        self.scopes.push(function_scope);
        self.known_string_values.push(HashMap::new());
        self.known_test262_realms.push(HashSet::new());
        for parameter in &mut function.params {
            if let Some(default) = parameter.default.take() {
                parameter.default = Some(self.lower_expression(default)?);
            }
        }
        function.body = self.lower_statement_list(function.body)?;
        self.known_test262_realms.pop();
        self.known_string_values.pop();
        self.scopes.pop();
        Ok(function)
    }

    fn lower_synthetic_function(
        &mut self,
        mut function: FunctionDeclaration,
    ) -> Result<FunctionDeclaration> {
        function.top_level_binding = None;
        function.register_global = false;
        function.self_binding = None;

        let saved_scopes = std::mem::take(&mut self.scopes);
        self.scopes.push(self.global_scope.clone());
        self.known_string_values.push(HashMap::new());
        self.known_test262_realms.push(HashSet::new());
        let result = self.lower_function(function);
        self.known_test262_realms.pop();
        self.known_string_values.pop();
        self.scopes = saved_scopes;
        result
    }

    fn lower_statement_list(&mut self, statements: Vec<Statement>) -> Result<Vec<Statement>> {
        statements
            .into_iter()
            .map(|statement| self.lower_statement(statement))
            .collect()
    }

    fn lower_scoped_statement_list(
        &mut self,
        statements: Vec<Statement>,
        extra_bindings: impl IntoIterator<Item = String>,
    ) -> Result<Vec<Statement>> {
        let mut scope = collect_statement_bindings(statements.iter())
            .into_iter()
            .collect::<HashSet<_>>();
        scope.extend(extra_bindings);
        self.scopes.push(scope);
        self.known_string_values.push(HashMap::new());
        self.known_test262_realms.push(HashSet::new());
        let result = self.lower_statement_list(statements);
        self.known_test262_realms.pop();
        self.known_string_values.pop();
        self.scopes.pop();
        result
    }

    fn lower_statement(&mut self, statement: Statement) -> Result<Statement> {
        match statement {
            Statement::Declaration { body } => Ok(Statement::Declaration {
                body: self.lower_statement_list(body)?,
            }),
            Statement::Block { body } => Ok(Statement::Block {
                body: self.lower_scoped_statement_list(body, [])?,
            }),
            Statement::Labeled { labels, body } => Ok(Statement::Labeled {
                labels,
                body: self.lower_scoped_statement_list(body, [])?,
            }),
            Statement::Var { name, value } => {
                let value = self.lower_expression(value)?;
                let is_realm = self.is_test262_realm_value(&value);
                self.record_known_string_value(&name, self.resolve_compile_time_string(&value));
                self.record_known_test262_realm_value(&name, is_realm);
                Ok(Statement::Var { name, value })
            }
            Statement::Let {
                name,
                mutable,
                value,
            } => {
                let value = self.lower_expression(value)?;
                let is_realm = self.is_test262_realm_value(&value);
                self.record_known_string_value(&name, self.resolve_compile_time_string(&value));
                self.record_known_test262_realm_value(&name, is_realm);
                Ok(Statement::Let {
                    name,
                    mutable,
                    value,
                })
            }
            Statement::Assign { name, value } => {
                let value = self.lower_expression(value)?;
                let is_realm = self.is_test262_realm_value(&value);
                self.record_known_string_value(&name, self.resolve_compile_time_string(&value));
                self.record_known_test262_realm_value(&name, is_realm);
                Ok(Statement::Assign { name, value })
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => Ok(Statement::AssignMember {
                object: self.lower_expression(object)?,
                property: self.lower_expression(property)?,
                value: self.lower_expression(value)?,
            }),
            Statement::Print { values } => Ok(Statement::Print {
                values: values
                    .into_iter()
                    .map(|value| self.lower_expression(value))
                    .collect::<Result<Vec<_>>>()?,
            }),
            Statement::Expression(value) => {
                Ok(Statement::Expression(self.lower_expression(value)?))
            }
            Statement::Throw(value) => Ok(Statement::Throw(self.lower_expression(value)?)),
            Statement::Return(value) => Ok(Statement::Return(self.lower_expression(value)?)),
            Statement::Break { label } => Ok(Statement::Break { label }),
            Statement::Continue { label } => Ok(Statement::Continue { label }),
            Statement::Yield { value } => Ok(Statement::Yield {
                value: self.lower_expression(value)?,
            }),
            Statement::YieldDelegate { value } => Ok(Statement::YieldDelegate {
                value: self.lower_expression(value)?,
            }),
            Statement::With { object, body } => Ok(Statement::With {
                object: self.lower_expression(object)?,
                body: self.lower_scoped_statement_list(body, [])?,
            }),
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => Ok(Statement::If {
                condition: self.lower_expression(condition)?,
                then_branch: self.lower_scoped_statement_list(then_branch, [])?,
                else_branch: self.lower_scoped_statement_list(else_branch, [])?,
            }),
            Statement::Try {
                body,
                catch_binding,
                catch_setup,
                catch_body,
            } => {
                let body = self.lower_scoped_statement_list(body, [])?;

                let mut catch_bindings =
                    collect_statement_bindings(catch_setup.iter().chain(catch_body.iter()));
                if let Some(binding) = &catch_binding {
                    catch_bindings.push(binding.clone());
                }

                let catch_setup =
                    self.lower_scoped_statement_list(catch_setup, catch_bindings.iter().cloned())?;
                let catch_body = self.lower_scoped_statement_list(catch_body, catch_bindings)?;

                Ok(Statement::Try {
                    body,
                    catch_binding,
                    catch_setup,
                    catch_body,
                })
            }
            Statement::Switch {
                labels,
                bindings,
                discriminant,
                cases,
            } => {
                self.scopes
                    .push(bindings.iter().cloned().collect::<HashSet<_>>());
                self.known_string_values.push(HashMap::new());
                self.known_test262_realms.push(HashSet::new());
                let result = (|| -> Result<Vec<SwitchCase>> {
                    cases
                        .into_iter()
                        .map(|case| {
                            Ok(SwitchCase {
                                test: match case.test {
                                    Some(test) => Some(self.lower_expression(test)?),
                                    None => None,
                                },
                                body: self.lower_statement_list(case.body)?,
                            })
                        })
                        .collect()
                })();
                self.known_test262_realms.pop();
                self.known_string_values.pop();
                self.scopes.pop();

                Ok(Statement::Switch {
                    labels,
                    bindings,
                    discriminant: self.lower_expression(discriminant)?,
                    cases: result?,
                })
            }
            Statement::For {
                labels,
                init,
                per_iteration_bindings,
                condition,
                update,
                break_hook,
                body,
            } => {
                let mut loop_bindings = collect_statement_bindings(init.iter());
                loop_bindings.extend(per_iteration_bindings.iter().cloned());

                self.scopes
                    .push(loop_bindings.into_iter().collect::<HashSet<_>>());
                self.known_string_values.push(HashMap::new());
                self.known_test262_realms.push(HashSet::new());
                let result = (|| -> Result<_> {
                    Ok(Statement::For {
                        labels,
                        init: self.lower_statement_list(init)?,
                        per_iteration_bindings,
                        condition: match condition {
                            Some(condition) => Some(self.lower_expression(condition)?),
                            None => None,
                        },
                        update: match update {
                            Some(update) => Some(self.lower_expression(update)?),
                            None => None,
                        },
                        break_hook: match break_hook {
                            Some(break_hook) => Some(self.lower_expression(break_hook)?),
                            None => None,
                        },
                        body: self.lower_statement_list(body)?,
                    })
                })();
                self.known_test262_realms.pop();
                self.known_string_values.pop();
                self.scopes.pop();
                result
            }
            Statement::While {
                labels,
                condition,
                break_hook,
                body,
            } => Ok(Statement::While {
                labels,
                condition: self.lower_expression(condition)?,
                break_hook: match break_hook {
                    Some(break_hook) => Some(self.lower_expression(break_hook)?),
                    None => None,
                },
                body: self.lower_scoped_statement_list(body, [])?,
            }),
            Statement::DoWhile {
                labels,
                condition,
                break_hook,
                body,
            } => Ok(Statement::DoWhile {
                labels,
                condition: self.lower_expression(condition)?,
                break_hook: match break_hook {
                    Some(break_hook) => Some(self.lower_expression(break_hook)?),
                    None => None,
                },
                body: self.lower_scoped_statement_list(body, [])?,
            }),
        }
    }

    fn lower_expression(&mut self, expression: Expression) -> Result<Expression> {
        match expression {
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::NewTarget
            | Expression::Identifier(_)
            | Expression::This
            | Expression::Sent => Ok(expression),
            Expression::Array(elements) => Ok(Expression::Array(
                elements
                    .into_iter()
                    .map(|element| match element {
                        ArrayElement::Expression(expression) => {
                            Ok(ArrayElement::Expression(self.lower_expression(expression)?))
                        }
                        ArrayElement::Spread(expression) => {
                            Ok(ArrayElement::Spread(self.lower_expression(expression)?))
                        }
                    })
                    .collect::<Result<Vec<_>>>()?,
            )),
            Expression::Object(entries) => Ok(Expression::Object(
                entries
                    .into_iter()
                    .map(|entry| match entry {
                        ObjectEntry::Data { key, value } => Ok(ObjectEntry::Data {
                            key: self.lower_expression(key)?,
                            value: self.lower_expression(value)?,
                        }),
                        ObjectEntry::Getter { key, getter } => Ok(ObjectEntry::Getter {
                            key: self.lower_expression(key)?,
                            getter: self.lower_expression(getter)?,
                        }),
                        ObjectEntry::Setter { key, setter } => Ok(ObjectEntry::Setter {
                            key: self.lower_expression(key)?,
                            setter: self.lower_expression(setter)?,
                        }),
                        ObjectEntry::Spread(expression) => {
                            Ok(ObjectEntry::Spread(self.lower_expression(expression)?))
                        }
                    })
                    .collect::<Result<Vec<_>>>()?,
            )),
            Expression::Member { object, property } => Ok(Expression::Member {
                object: Box::new(self.lower_expression(*object)?),
                property: Box::new(self.lower_expression(*property)?),
            }),
            Expression::SuperMember { property } => Ok(Expression::SuperMember {
                property: Box::new(self.lower_expression(*property)?),
            }),
            Expression::Assign { name, value } => {
                let value = self.lower_expression(*value)?;
                let is_realm = self.is_test262_realm_value(&value);
                self.record_known_string_value(&name, self.resolve_compile_time_string(&value));
                self.record_known_test262_realm_value(&name, is_realm);
                Ok(Expression::Assign {
                    name,
                    value: Box::new(value),
                })
            }
            Expression::AssignMember {
                object,
                property,
                value,
            } => Ok(Expression::AssignMember {
                object: Box::new(self.lower_expression(*object)?),
                property: Box::new(self.lower_expression(*property)?),
                value: Box::new(self.lower_expression(*value)?),
            }),
            Expression::AssignSuperMember { property, value } => {
                Ok(Expression::AssignSuperMember {
                    property: Box::new(self.lower_expression(*property)?),
                    value: Box::new(self.lower_expression(*value)?),
                })
            }
            Expression::Await(expression) => Ok(Expression::Await(Box::new(
                self.lower_expression(*expression)?,
            ))),
            Expression::EnumerateKeys(expression) => Ok(Expression::EnumerateKeys(Box::new(
                self.lower_expression(*expression)?,
            ))),
            Expression::GetIterator(expression) => Ok(Expression::GetIterator(Box::new(
                self.lower_expression(*expression)?,
            ))),
            Expression::IteratorClose(expression) => Ok(Expression::IteratorClose(Box::new(
                self.lower_expression(*expression)?,
            ))),
            Expression::Unary { op, expression } => Ok(Expression::Unary {
                op,
                expression: Box::new(self.lower_expression(*expression)?),
            }),
            Expression::Binary { op, left, right } => Ok(Expression::Binary {
                op,
                left: Box::new(self.lower_expression(*left)?),
                right: Box::new(self.lower_expression(*right)?),
            }),
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => Ok(Expression::Conditional {
                condition: Box::new(self.lower_expression(*condition)?),
                then_expression: Box::new(self.lower_expression(*then_expression)?),
                else_expression: Box::new(self.lower_expression(*else_expression)?),
            }),
            Expression::Sequence(expressions) => Ok(Expression::Sequence(
                expressions
                    .into_iter()
                    .map(|expression| self.lower_expression(expression))
                    .collect::<Result<Vec<_>>>()?,
            )),
            Expression::Call { callee, arguments } => {
                let callee = self.lower_expression(*callee)?;
                let arguments = self.lower_arguments(arguments)?;
                if let Some(lowered) =
                    self.try_lower_static_function_constructor(&callee, &arguments)?
                {
                    return Ok(lowered);
                }
                if let Some(lowered) =
                    self.try_lower_static_member_eval_function_expression(&callee, &arguments)?
                {
                    return Ok(lowered);
                }
                Ok(Expression::Call {
                    callee: Box::new(callee),
                    arguments,
                })
            }
            Expression::SuperCall { callee, arguments } => Ok(Expression::SuperCall {
                callee: Box::new(self.lower_expression(*callee)?),
                arguments: self.lower_arguments(arguments)?,
            }),
            Expression::New { callee, arguments } => {
                let callee = self.lower_expression(*callee)?;
                let arguments = self.lower_arguments(arguments)?;
                if let Some(lowered) =
                    self.try_lower_static_function_constructor(&callee, &arguments)?
                {
                    return Ok(lowered);
                }
                Ok(Expression::New {
                    callee: Box::new(callee),
                    arguments,
                })
            }
            Expression::Update { .. } => Ok(expression),
        }
    }

    fn lower_arguments(&mut self, arguments: Vec<CallArgument>) -> Result<Vec<CallArgument>> {
        arguments
            .into_iter()
            .map(|argument| match argument {
                CallArgument::Expression(expression) => {
                    Ok(CallArgument::Expression(self.lower_expression(expression)?))
                }
                CallArgument::Spread(expression) => {
                    Ok(CallArgument::Spread(self.lower_expression(expression)?))
                }
            })
            .collect()
    }

    fn record_known_string_value(&mut self, name: &str, value: Option<String>) {
        if let Some(scope) = self.known_string_values.last_mut() {
            if let Some(value) = value {
                scope.insert(name.to_string(), value);
            } else {
                scope.remove(name);
            }
        }
    }

    fn lookup_known_string_value(&self, name: &str) -> Option<String> {
        self.known_string_values
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).cloned())
    }

    fn record_known_test262_realm_value(&mut self, name: &str, is_realm: bool) {
        if let Some(scope) = self.known_test262_realms.last_mut() {
            if is_realm {
                scope.insert(name.to_string());
            } else {
                scope.remove(name);
            }
        }
    }

    pub(super) fn lookup_known_test262_realm_value(&self, name: &str) -> bool {
        self.known_test262_realms
            .iter()
            .rev()
            .any(|scope| scope.contains(name))
    }

    pub(super) fn is_test262_realm_value(&self, expression: &Expression) -> bool {
        match expression {
            Expression::Identifier(name) => self.lookup_known_test262_realm_value(name),
            Expression::Call { callee, arguments } if arguments.is_empty() => {
                matches!(
                    callee.as_ref(),
                    Expression::Member { object, property }
                        if self.is_global_identifier(object, "$262")
                            && self.is_string_literal(property, "createRealm")
                )
            }
            _ => false,
        }
    }

    pub(super) fn is_test262_realm_global_value(&self, expression: &Expression) -> bool {
        matches!(
            expression,
            Expression::Member { object, property }
                if self.is_test262_realm_value(object)
                    && self.is_string_literal(property, "global")
        )
    }

    pub(super) fn resolve_compile_time_string(&self, expression: &Expression) -> Option<String> {
        match expression {
            Expression::String(text) => Some(text.clone()),
            Expression::Identifier(name) => self.lookup_known_string_value(name),
            Expression::Binary {
                op: crate::ir::hir::BinaryOp::Add,
                left,
                right,
            } => Some(format!(
                "{}{}",
                self.resolve_compile_time_string(left)?,
                self.resolve_compile_time_string(right)?
            )),
            _ => None,
        }
    }
}
