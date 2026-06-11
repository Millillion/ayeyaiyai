use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn find_iterator_source_expression_in_statements(
        statements: &[Statement],
        iterator_name: &str,
    ) -> Option<Expression> {
        for statement in statements {
            match statement {
                Statement::Declaration { body }
                | Statement::Block { body }
                | Statement::Labeled { body, .. }
                | Statement::With { body, .. } => {
                    if let Some(iterated) =
                        Self::find_iterator_source_expression_in_statements(body, iterator_name)
                    {
                        return Some(iterated);
                    }
                }
                Statement::If {
                    then_branch,
                    else_branch,
                    ..
                } => {
                    if let Some(iterated) = Self::find_iterator_source_expression_in_statements(
                        then_branch,
                        iterator_name,
                    ) {
                        return Some(iterated);
                    }
                    if let Some(iterated) = Self::find_iterator_source_expression_in_statements(
                        else_branch,
                        iterator_name,
                    ) {
                        return Some(iterated);
                    }
                }
                Statement::Try {
                    body,
                    catch_setup,
                    catch_body,
                    ..
                } => {
                    if let Some(iterated) =
                        Self::find_iterator_source_expression_in_statements(body, iterator_name)
                    {
                        return Some(iterated);
                    }
                    if let Some(iterated) = Self::find_iterator_source_expression_in_statements(
                        catch_setup,
                        iterator_name,
                    ) {
                        return Some(iterated);
                    }
                    if let Some(iterated) = Self::find_iterator_source_expression_in_statements(
                        catch_body,
                        iterator_name,
                    ) {
                        return Some(iterated);
                    }
                }
                Statement::Switch { cases, .. } => {
                    for case in cases {
                        if let Some(iterated) = Self::find_iterator_source_expression_in_statements(
                            &case.body,
                            iterator_name,
                        ) {
                            return Some(iterated);
                        }
                    }
                }
                Statement::For { init, body, .. } => {
                    if let Some(iterated) =
                        Self::find_iterator_source_expression_in_statements(init, iterator_name)
                    {
                        return Some(iterated);
                    }
                    if let Some(iterated) =
                        Self::find_iterator_source_expression_in_statements(body, iterator_name)
                    {
                        return Some(iterated);
                    }
                }
                Statement::While { body, .. } | Statement::DoWhile { body, .. } => {
                    if let Some(iterated) =
                        Self::find_iterator_source_expression_in_statements(body, iterator_name)
                    {
                        return Some(iterated);
                    }
                }
                Statement::Var { name, value }
                | Statement::Let { name, value, .. }
                | Statement::Assign { name, value }
                    if name == iterator_name =>
                {
                    if let Expression::GetIterator(iterated) = value {
                        return Some((**iterated).clone());
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn collect_binding_definition_expressions_in_statements<'s>(
        statements: &'s [Statement],
        binding_name: &str,
        definitions: &mut Vec<&'s Expression>,
    ) {
        for statement in statements {
            match statement {
                Statement::Declaration { body }
                | Statement::Block { body }
                | Statement::Labeled { body, .. }
                | Statement::With { body, .. }
                | Statement::While { body, .. }
                | Statement::DoWhile { body, .. } => {
                    Self::collect_binding_definition_expressions_in_statements(
                        body,
                        binding_name,
                        definitions,
                    );
                }
                Statement::If {
                    then_branch,
                    else_branch,
                    ..
                } => {
                    Self::collect_binding_definition_expressions_in_statements(
                        then_branch,
                        binding_name,
                        definitions,
                    );
                    Self::collect_binding_definition_expressions_in_statements(
                        else_branch,
                        binding_name,
                        definitions,
                    );
                }
                Statement::Try {
                    body,
                    catch_setup,
                    catch_body,
                    ..
                } => {
                    Self::collect_binding_definition_expressions_in_statements(
                        body,
                        binding_name,
                        definitions,
                    );
                    Self::collect_binding_definition_expressions_in_statements(
                        catch_setup,
                        binding_name,
                        definitions,
                    );
                    Self::collect_binding_definition_expressions_in_statements(
                        catch_body,
                        binding_name,
                        definitions,
                    );
                }
                Statement::Switch { cases, .. } => {
                    for case in cases {
                        Self::collect_binding_definition_expressions_in_statements(
                            &case.body,
                            binding_name,
                            definitions,
                        );
                    }
                }
                Statement::For { init, body, .. } => {
                    Self::collect_binding_definition_expressions_in_statements(
                        init,
                        binding_name,
                        definitions,
                    );
                    Self::collect_binding_definition_expressions_in_statements(
                        body,
                        binding_name,
                        definitions,
                    );
                }
                Statement::Var { name, value }
                | Statement::Let { name, value, .. }
                | Statement::Assign { name, value }
                    if name == binding_name =>
                {
                    definitions.push(value);
                }
                _ => {}
            }
        }
    }

    /// Resolves the single definition expression for a binding inside a
    /// function body, giving up when the binding is reassigned with distinct
    /// values (the data flow would then be ambiguous).
    fn find_unique_binding_definition_expression_in_statements(
        statements: &[Statement],
        binding_name: &str,
    ) -> Option<Expression> {
        let mut definitions = Vec::new();
        Self::collect_binding_definition_expressions_in_statements(
            statements,
            binding_name,
            &mut definitions,
        );
        let (first, rest) = definitions.split_first()?;
        if rest
            .iter()
            .all(|definition| static_expression_matches(definition, first))
        {
            Some((*first).clone())
        } else {
            None
        }
    }

    /// Expands an iterated-source expression through the lowered iterator
    /// machinery of a function body: hidden temporaries are chased to their
    /// definitions, and `step.value` reads of a `next()` step over a static
    /// array literal expand to the literal's element expressions. The result
    /// is the set of candidate expressions whose iterator protocol methods a
    /// destructuring or close of `value` may invoke.
    pub(in crate::backend::direct_wasm) fn iterator_iterated_value_candidates_in_statements(
        statements: &[Statement],
        value: &Expression,
        depth: usize,
    ) -> Vec<Expression> {
        if depth >= 8 {
            return vec![value.clone()];
        }
        match value {
            Expression::Identifier(name) => {
                match Self::find_unique_binding_definition_expression_in_statements(
                    statements, name,
                ) {
                    Some(definition) => Self::iterator_iterated_value_candidates_in_statements(
                        statements,
                        &definition,
                        depth + 1,
                    ),
                    None => vec![value.clone()],
                }
            }
            Expression::GetIterator(inner) | Expression::Await(inner) => {
                Self::iterator_iterated_value_candidates_in_statements(statements, inner, depth + 1)
            }
            Expression::Member { object, property }
                if matches!(
                    property.as_ref(),
                    Expression::String(name) if name == "value"
                ) =>
            {
                let Expression::Identifier(step_name) = object.as_ref() else {
                    return vec![value.clone()];
                };
                let Some(step_definition) =
                    Self::find_unique_binding_definition_expression_in_statements(
                        statements, step_name,
                    )
                else {
                    return vec![value.clone()];
                };
                let Expression::Call { callee, .. } = &step_definition else {
                    return vec![value.clone()];
                };
                let Expression::Member {
                    object: iterator_object,
                    property: method,
                } = callee.as_ref()
                else {
                    return vec![value.clone()];
                };
                if !matches!(
                    method.as_ref(),
                    Expression::String(name) if name == "next"
                ) {
                    return vec![value.clone()];
                }
                let iterated_candidates = Self::iterator_iterated_value_candidates_in_statements(
                    statements,
                    iterator_object,
                    depth + 1,
                );
                let mut results = Vec::new();
                for candidate in iterated_candidates {
                    let Expression::Array(elements) = candidate else {
                        return vec![value.clone()];
                    };
                    for element in elements {
                        match element {
                            ArrayElement::Expression(element) => {
                                results.extend(
                                    Self::iterator_iterated_value_candidates_in_statements(
                                        statements,
                                        &element,
                                        depth + 1,
                                    ),
                                );
                            }
                            ArrayElement::Spread(_) => {
                                return vec![value.clone()];
                            }
                        }
                    }
                }
                if results.is_empty() {
                    vec![value.clone()]
                } else {
                    results
                }
            }
            _ => vec![value.clone()],
        }
    }

    /// Resolves the iterator-protocol method bindings (`next`/`return`/
    /// `throw`) that an operation on a hidden iterator temporary may invoke,
    /// following the lowered data-flow chain back to statically known iterated
    /// sources.
    pub(in crate::backend::direct_wasm) fn resolve_iterator_protocol_method_bindings_in_function(
        &self,
        iterator_name: &str,
        method_name: &str,
        current_function_name: Option<&str>,
    ) -> Vec<LocalFunctionBinding> {
        let Some(function_name) = current_function_name else {
            return Vec::new();
        };
        let Some(function) = self.resolve_registered_function_declaration(function_name) else {
            return Vec::new();
        };
        let Some(iterated) =
            Self::find_iterator_source_expression_in_statements(&function.body, iterator_name)
        else {
            return Vec::new();
        };
        let candidates =
            Self::iterator_iterated_value_candidates_in_statements(&function.body, &iterated, 0);
        let mut bindings = Vec::new();
        for candidate in candidates {
            let iterator_call = Expression::Call {
                callee: Box::new(Expression::Member {
                    object: Box::new(candidate),
                    property: Box::new(symbol_iterator_expression()),
                }),
                arguments: Vec::new(),
            };
            bindings.extend(
                self.inherited_member_function_bindings(&iterator_call)
                    .into_iter()
                    .filter(|binding| binding.property == method_name)
                    .map(|binding| binding.binding),
            );
        }
        bindings
    }

    /// Resolves the single user-defined iterator-protocol method that an
    /// operation on a hidden iterator temporary can invoke. Returns `None`
    /// when the candidates are ambiguous (multiple distinct user functions) or
    /// include builtins, so callers only commit to a statically-chosen callee
    /// when it is unique.
    pub(in crate::backend::direct_wasm) fn resolve_unique_iterator_protocol_user_method_in_function(
        &self,
        iterator_name: &str,
        method_name: &str,
        current_function_name: Option<&str>,
    ) -> Option<LocalFunctionBinding> {
        let bindings = self.resolve_iterator_protocol_method_bindings_in_function(
            iterator_name,
            method_name,
            current_function_name,
        );
        if bindings.is_empty() {
            return None;
        }
        let mut user_names = Vec::new();
        for binding in &bindings {
            let LocalFunctionBinding::User(function_name) = binding else {
                return None;
            };
            if !user_names.contains(function_name) {
                user_names.push(function_name.clone());
            }
        }
        let [function_name] = user_names.as_slice() else {
            return None;
        };
        Some(LocalFunctionBinding::User(function_name.clone()))
    }
}
