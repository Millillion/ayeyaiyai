use super::*;
use crate::ir::hir::SPREAD_ITERATE_HELPER_NAME;
use crate::ir::visit::{Visitor, walk_call_argument, walk_program};

fn program_uses_call_spread(program: &Program) -> bool {
    #[derive(Default)]
    struct SpreadCallArgumentFinder {
        found: bool,
    }

    impl Visitor for SpreadCallArgumentFinder {
        fn visit_call_argument(&mut self, argument: &CallArgument) {
            if matches!(argument, CallArgument::Spread(_)) {
                self.found = true;
            }
            walk_call_argument(self, argument);
        }
    }

    let mut finder = SpreadCallArgumentFinder::default();
    walk_program(&mut finder, program);
    finder.found
}

// The helper body is hand-built rather than lowered from source on purpose:
// the canonical `__ayy_for_of_*` binding names produced by the for-of
// lowering are recognized by the backend's static iterator machinery, which
// would specialize the loop away and drop the observable GetIterator/next()
// protocol effects this helper exists to preserve. The distinct
// `__ayy_spread_iterate_*` names keep the loop opaque so it always runs at
// runtime. `Var` statements are used (instead of `Let`) so the bindings are
// plain function-scoped locals without TDZ-initialization tracking.
fn spread_iterate_helper_declaration() -> FunctionDeclaration {
    let source_name = "__ayy_spread_iterate_source";
    let result_name = "__ayy_spread_iterate_result";
    let iterator_name = "__ayy_spread_iterate_iter";
    let step_name = "__ayy_spread_iterate_step";
    let value_name = "__ayy_spread_iterate_value";

    let body = vec![
        Statement::Var {
            name: result_name.to_string(),
            value: Expression::Array(Vec::new()),
        },
        Statement::Var {
            name: iterator_name.to_string(),
            value: Expression::GetIterator(Box::new(Expression::Identifier(
                source_name.to_string(),
            ))),
        },
        Statement::For {
            labels: Vec::new(),
            init: Vec::new(),
            per_iteration_bindings: Vec::new(),
            condition: Some(Expression::Bool(true)),
            update: None,
            break_hook: None,
            body: vec![
                Statement::Var {
                    name: step_name.to_string(),
                    value: Expression::Call {
                        callee: Box::new(Expression::Member {
                            object: Box::new(Expression::Identifier(iterator_name.to_string())),
                            property: Box::new(Expression::String("next".to_string())),
                        }),
                        arguments: Vec::new(),
                    },
                },
                Statement::If {
                    condition: Expression::Member {
                        object: Box::new(Expression::Identifier(step_name.to_string())),
                        property: Box::new(Expression::String("done".to_string())),
                    },
                    then_branch: vec![Statement::Break { label: None }],
                    else_branch: Vec::new(),
                },
                Statement::Var {
                    name: value_name.to_string(),
                    value: Expression::Member {
                        object: Box::new(Expression::Identifier(step_name.to_string())),
                        property: Box::new(Expression::String("value".to_string())),
                    },
                },
                Statement::Expression(Expression::Call {
                    callee: Box::new(Expression::Member {
                        object: Box::new(Expression::Identifier(result_name.to_string())),
                        property: Box::new(Expression::String("push".to_string())),
                    }),
                    arguments: vec![CallArgument::Expression(Expression::Identifier(
                        value_name.to_string(),
                    ))],
                }),
            ],
        },
        Statement::Return(Expression::Identifier(result_name.to_string())),
    ];

    FunctionDeclaration {
        name: SPREAD_ITERATE_HELPER_NAME.to_string(),
        top_level_binding: None,
        params: vec![Parameter {
            name: source_name.to_string(),
            default: None,
            rest: false,
        }],
        body,
        register_global: false,
        kind: FunctionKind::Ordinary,
        self_binding: None,
        mapped_arguments: false,
        strict: true,
        lexical_this: false,
        constructible: false,
        derived_constructor: false,
        direct_eval_in_class_field_initializer: false,
        length: 1,
        synthetic_capture_bindings: Vec::new(),
        immutable_class_bindings: Vec::new(),
        private_brand_binding: None,
    }
}

impl Lowerer {
    pub(crate) fn with_source_text(source_text: String) -> Self {
        Self {
            source_text: Some(source_text),
            ..Self::default()
        }
    }

    pub(crate) fn current_immutable_class_bindings(&self) -> Vec<String> {
        let mut bindings = Vec::new();
        for binding in &self.immutable_class_binding_stack {
            if !bindings.contains(binding) {
                bindings.push(binding.clone());
            }
        }
        bindings
    }

    pub(crate) fn current_immutable_class_bindings_with(&self, binding: &str) -> Vec<String> {
        let mut bindings = self.current_immutable_class_bindings();
        if !bindings.iter().any(|existing| existing == binding) {
            bindings.push(binding.to_string());
        }
        bindings
    }

    pub(crate) fn source_span_snippet(&self, span: Span) -> Option<&str> {
        let source = self.source_text.as_deref()?;
        if span.lo.is_dummy() || span.hi.is_dummy() {
            return None;
        }
        let start = span.lo.0.saturating_sub(1) as usize;
        let end = span.hi.0.saturating_sub(1) as usize;
        source.get(start..end)
    }

    pub(crate) fn private_name_key(&self, private_name: &swc_ecma_ast::PrivateName) -> String {
        let fallback = private_name.name.to_string();
        let Some(source_name) = self
            .source_span_snippet(private_name.span)
            .and_then(Self::private_name_source_identifier)
        else {
            return fallback;
        };

        Self::decode_identifier_unicode_escapes(source_name).unwrap_or(fallback)
    }

    fn private_name_source_identifier(source: &str) -> Option<&str> {
        let source = source.trim();
        let source = source.strip_prefix('#')?;
        let mut end = source.len();
        let mut chars = source.char_indices().peekable();

        while let Some((index, character)) = chars.next() {
            match character {
                '\\' => {
                    let Some((_, 'u')) = chars.next() else {
                        end = index;
                        break;
                    };

                    if matches!(chars.peek(), Some((_, '{'))) {
                        chars.next();
                        let mut closed = false;
                        for (_, escaped_character) in chars.by_ref() {
                            if escaped_character == '}' {
                                closed = true;
                                break;
                            }
                        }
                        if !closed {
                            end = index;
                            break;
                        }
                    } else {
                        for _ in 0..4 {
                            if chars.next().is_none() {
                                end = index;
                                break;
                            }
                        }
                    }
                }
                '(' | ')' | '=' | ';' | ',' | ':' | '[' | ']' | '{' | '}' => {
                    end = index;
                    break;
                }
                character if character.is_whitespace() => {
                    end = index;
                    break;
                }
                _ => {}
            }
        }

        source.get(..end)
    }

    fn decode_identifier_unicode_escapes(identifier: &str) -> Option<String> {
        if !identifier.contains("\\u") {
            return Some(identifier.to_string());
        }

        let mut decoded = String::with_capacity(identifier.len());
        let mut chars = identifier.chars().peekable();

        while let Some(character) = chars.next() {
            if character != '\\' {
                decoded.push(character);
                continue;
            }

            if chars.next()? != 'u' {
                return None;
            }

            let mut digits = String::new();
            if matches!(chars.peek(), Some('{')) {
                chars.next();
                loop {
                    let escaped_character = chars.next()?;
                    if escaped_character == '}' {
                        break;
                    }
                    if !escaped_character.is_ascii_hexdigit() {
                        return None;
                    }
                    digits.push(escaped_character);
                }
                if digits.is_empty() {
                    return None;
                }
            } else {
                for _ in 0..4 {
                    let digit = chars.next()?;
                    if !digit.is_ascii_hexdigit() {
                        return None;
                    }
                    digits.push(digit);
                }
            }

            let code_point = u32::from_str_radix(&digits, 16).ok()?;
            decoded.push(char::from_u32(code_point)?);
        }

        Some(decoded)
    }

    fn array_pattern_inner_source(&self, array: &swc_ecma_ast::ArrayPat) -> Option<&str> {
        let source = self.source_text.as_deref()?;
        if array.span.lo.is_dummy() {
            return None;
        }
        let mut start = array.span.lo.0.saturating_sub(1) as usize;
        start += source.get(start..)?.find('[')?;
        let mut depth = 0usize;
        let mut inner_start = None;
        for (relative_index, character) in source.get(start..)?.char_indices() {
            let index = start + relative_index;
            match character {
                '[' => {
                    depth = depth.saturating_add(1);
                    if inner_start.is_none() {
                        inner_start = Some(index + character.len_utf8());
                    }
                }
                ']' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return source.get(inner_start?..index);
                    }
                }
                _ => {}
            }
        }
        None
    }

    pub(crate) fn pure_array_pattern_elision_count(&self, array: &swc_ecma_ast::ArrayPat) -> usize {
        if !array.elems.is_empty() {
            return 0;
        }
        let Some(inner) = self.array_pattern_inner_source(array).or_else(|| {
            self.source_span_snippet(array.span).and_then(|snippet| {
                snippet
                    .strip_prefix('[')
                    .and_then(|text| text.strip_suffix(']'))
            })
        }) else {
            return 0;
        };
        if inner
            .chars()
            .all(|character| character.is_whitespace() || character == ',')
        {
            inner.chars().filter(|&character| character == ',').count()
        } else {
            0
        }
    }

    pub(crate) fn array_pattern_has_non_elision_trailing_comma(
        &self,
        array: &swc_ecma_ast::ArrayPat,
    ) -> bool {
        let Some(inner) = self.array_pattern_inner_source(array).or_else(|| {
            self.source_span_snippet(array.span).and_then(|snippet| {
                snippet
                    .strip_prefix('[')
                    .and_then(|text| text.strip_suffix(']'))
            })
        }) else {
            return false;
        };
        let trimmed = inner.trim_end();
        let Some(before_comma) = trimmed.strip_suffix(',') else {
            return false;
        };
        before_comma
            .chars()
            .rev()
            .find(|character| !character.is_whitespace())
            .is_some_and(|character| character != ',')
    }

    pub(crate) fn array_pattern_trailing_elision_count(
        &self,
        array: &swc_ecma_ast::ArrayPat,
    ) -> usize {
        let Some(inner) = self.array_pattern_inner_source(array).or_else(|| {
            self.source_span_snippet(array.span).and_then(|snippet| {
                snippet
                    .strip_prefix('[')
                    .and_then(|text| text.strip_suffix(']'))
            })
        }) else {
            return 0;
        };
        let comma_count = inner.chars().filter(|&character| character == ',').count();
        comma_count.saturating_sub(array.elems.len())
    }

    pub(crate) fn lower_program(&mut self, program: &SwcProgram) -> Result<Program> {
        let mut statements = Vec::new();
        let strict_mode = match program {
            SwcProgram::Script(script) => script_has_use_strict_directive(&script.body),
            SwcProgram::Module(_) => true,
        };
        self.strict_modes.push(strict_mode);
        self.module_mode = matches!(program, SwcProgram::Module(_));

        match program {
            SwcProgram::Script(script) => {
                let scope_bindings = collect_direct_statement_lexical_bindings(&script.body)?;
                self.push_binding_scope(scope_bindings);
                let lowered = self.lower_top_level_statements(script.body.iter(), &mut statements);
                self.pop_binding_scope();
                lowered?
            }
            SwcProgram::Module(module) => {
                self.push_this_replacement(Some(Expression::Undefined));
                for item in &module.body {
                    match item {
                        ModuleItem::Stmt(statement) => {
                            self.lower_top_level_statement(statement, &mut statements)?
                        }
                        ModuleItem::ModuleDecl(module_declaration) => {
                            self.lower_module_declaration(module_declaration, &mut statements)?
                        }
                    }
                }
                self.pop_this_replacement();
            }
        }

        self.strict_modes.pop();
        self.module_mode = false;
        self.current_module_path = None;
        self.module_index_lookup.clear();
        self.dynamic_import_specifier_lookup.clear();

        Ok(self.finish_program(statements, strict_mode))
    }

    pub(crate) fn finish_program(&mut self, statements: Vec<Statement>, strict: bool) -> Program {
        self.module_mode = false;
        self.current_module_path = None;
        self.module_index_lookup.clear();
        self.dynamic_import_specifier_lookup.clear();

        let mut functions = Vec::new();
        let mut seen = HashSet::new();
        for function in std::mem::take(&mut self.functions).into_iter().rev() {
            if seen.insert(function.name.clone()) {
                functions.push(function);
            }
        }
        functions.reverse();

        let mut program = Program {
            strict,
            functions,
            statements,
        };
        if !seen.contains(SPREAD_ITERATE_HELPER_NAME) && program_uses_call_spread(&program) {
            program.functions.push(spread_iterate_helper_declaration());
        }
        program
    }

    pub(crate) fn fresh_temporary_name(&mut self, prefix: &str) -> String {
        self.next_temporary_id += 1;
        format!("__ayy_{prefix}_{}", self.next_temporary_id)
    }

    pub(crate) fn fresh_scoped_binding_name(&mut self, name: &str) -> String {
        self.next_temporary_id += 1;
        format!("__ayy_scope${name}${}", self.next_temporary_id)
    }

    pub(crate) fn fresh_isolated_binding_name(&mut self, name: &str) -> String {
        self.next_temporary_id += 1;
        format!("__ayy_local${name}${}", self.next_temporary_id)
    }

    pub(crate) fn push_binding_scope(&mut self, names: Vec<String>) {
        self.push_binding_scope_with_mode(names, false);
    }

    pub(crate) fn push_renaming_binding_scope(&mut self, names: Vec<String>) {
        self.push_binding_scope_with_mode(names, true);
    }

    fn push_binding_scope_with_mode(&mut self, names: Vec<String>, force_renames: bool) {
        let mut scope = BindingScope::default();

        for name in names {
            if scope.names.contains(&name) {
                continue;
            }

            if force_renames {
                scope
                    .renames
                    .insert(name.clone(), self.fresh_isolated_binding_name(&name));
            } else if self.active_binding_counts.contains_key(&name) {
                scope
                    .renames
                    .insert(name.clone(), self.fresh_scoped_binding_name(&name));
            }

            *self.active_binding_counts.entry(name.clone()).or_insert(0) += 1;
            scope.names.push(name);
        }

        self.binding_scopes.push(scope);
    }

    pub(crate) fn pop_binding_scope(&mut self) {
        let Some(scope) = self.binding_scopes.pop() else {
            return;
        };

        for name in scope.names {
            let Some(count) = self.active_binding_counts.get_mut(&name) else {
                continue;
            };
            *count -= 1;
            if *count == 0 {
                self.active_binding_counts.remove(&name);
            }
        }
    }

    pub(crate) fn resolve_binding_name(&self, name: &str) -> String {
        for scope in self.binding_scopes.iter().rev() {
            if let Some(mapped) = scope.renames.get(name) {
                return mapped.clone();
            }
        }

        name.to_string()
    }

    pub(crate) fn lower_inside_with_scope<T>(
        &mut self,
        lower: impl FnOnce(&mut Self) -> Result<T>,
    ) -> Result<T> {
        self.with_scope_depth += 1;
        let result = lower(self);
        self.with_scope_depth -= 1;
        result
    }

    pub(crate) fn current_this_replacement(&self) -> Option<Expression> {
        self.this_replacements.last().cloned().flatten()
    }

    pub(crate) fn push_this_replacement(&mut self, replacement: Option<Expression>) {
        self.this_replacements.push(replacement);
    }

    pub(crate) fn pop_this_replacement(&mut self) {
        self.this_replacements.pop();
    }

    pub(crate) fn with_this_replacement<T>(
        &mut self,
        replacement: Option<Expression>,
        operation: impl FnOnce(&mut Self) -> Result<T>,
    ) -> Result<T> {
        self.push_this_replacement(replacement);
        let result = operation(self);
        self.pop_this_replacement();
        result
    }

    pub(crate) fn current_super_member_replacement(&self) -> Option<Expression> {
        self.super_member_replacements.last().cloned().flatten()
    }

    pub(crate) fn push_super_member_replacement(&mut self, replacement: Option<Expression>) {
        self.super_member_replacements.push(replacement);
    }

    pub(crate) fn pop_super_member_replacement(&mut self) {
        self.super_member_replacements.pop();
    }

    pub(crate) fn with_super_member_replacement<T>(
        &mut self,
        replacement: Option<Expression>,
        operation: impl FnOnce(&mut Self) -> Result<T>,
    ) -> Result<T> {
        self.push_super_member_replacement(replacement);
        let result = operation(self);
        self.pop_super_member_replacement();
        result
    }

    pub(crate) fn lower_dynamic_import_expression(
        &mut self,
        call: &swc_ecma_ast::CallExpr,
        phase: swc_ecma_ast::ImportPhase,
    ) -> Result<Expression> {
        ensure!(
            matches!(call.args.len(), 1 | 2),
            "dynamic import expects one or two arguments"
        );
        for argument in &call.args {
            ensure!(
                argument.spread.is_none(),
                "dynamic import does not support spread arguments"
            );
        }

        let argument = &call.args[0];
        let lowered_argument = if let Expr::Lit(Lit::Str(specifier)) = &*argument.expr {
            let specifier_text = specifier.value.to_string_lossy().to_string();
            let module_index = self
                .current_module_path
                .as_ref()
                .and_then(|module_path| resolve_module_specifier(module_path, &specifier_text).ok())
                .and_then(|resolved| self.module_index_lookup.get(&resolved).copied())
                .or_else(|| {
                    self.dynamic_import_specifier_lookup
                        .get(&specifier_text)
                        .copied()
                })
                .map(|module_index| module_index as f64)
                .unwrap_or(-1.0);
            Expression::Number(module_index)
        } else {
            self.lower_expression(&argument.expr)?
        };
        let mut arguments = vec![CallArgument::Expression(lowered_argument)];
        if let Some(options) = call.args.get(1) {
            arguments.push(CallArgument::Expression(
                self.lower_expression(&options.expr)?,
            ));
        } else if !self.dynamic_import_specifier_lookup.is_empty() {
            arguments.push(CallArgument::Expression(Expression::Undefined));
        }
        if !self.dynamic_import_specifier_lookup.is_empty() {
            arguments.push(CallArgument::Expression(Expression::Object(
                self.dynamic_import_specifier_lookup
                    .iter()
                    .map(|(specifier, module_index)| ObjectEntry::Data {
                        key: Expression::String(specifier.clone()),
                        value: Expression::Number(*module_index as f64),
                    })
                    .collect(),
            )));
        }
        let phase_marker = match phase {
            swc_ecma_ast::ImportPhase::Defer => Some("__ayy$importPhase$defer"),
            swc_ecma_ast::ImportPhase::Source => Some("__ayy$importPhase$source"),
            swc_ecma_ast::ImportPhase::Evaluation => None,
        };
        if let Some(phase_marker) = phase_marker {
            while arguments.len() < 3 {
                arguments.push(CallArgument::Expression(Expression::Undefined));
            }
            arguments.push(CallArgument::Expression(Expression::String(
                phase_marker.to_string(),
            )));
        }

        Ok(Expression::Call {
            callee: Box::new(Expression::Identifier("__ayyDynamicImport".to_string())),
            arguments,
        })
    }

    pub(crate) fn lower_private_name(
        &mut self,
        private_name: &swc_ecma_ast::PrivateName,
    ) -> Result<Expression> {
        let name = self.private_name_key(private_name);
        for (index, scope) in self.private_name_scopes.iter().enumerate().rev() {
            if let Some(mapped) = scope.get(&name) {
                if let Some(brand_binding) = self
                    .private_name_brand_scopes
                    .get(index)
                    .and_then(|scope| scope.get(&name))
                    .cloned()
                    && self.private_brand_capture_suppression_depth == 0
                    && let Some(captures) = self.pending_private_brand_captures.last_mut()
                {
                    captures.insert(brand_binding);
                }
                return Ok(Expression::String(mapped.clone()));
            }
        }

        bail!("unsupported private name reference: #{name}")
    }

    pub(crate) fn lower_private_name_without_capture(
        &mut self,
        private_name: &swc_ecma_ast::PrivateName,
    ) -> Result<Expression> {
        self.private_brand_capture_suppression_depth += 1;
        let lowered = self.lower_private_name(private_name);
        self.private_brand_capture_suppression_depth -= 1;
        lowered
    }

    pub(crate) fn class_private_name_map(
        &self,
        class: &Class,
        binding_name: &str,
    ) -> HashMap<String, String> {
        let mut names = HashMap::new();
        for member in &class.body {
            match member {
                ClassMember::PrivateProp(property) => {
                    let name = self.private_name_key(&property.key);
                    names.insert(name.clone(), format!("__ayy$private${binding_name}${name}"));
                }
                ClassMember::PrivateMethod(method) => {
                    let name = self.private_name_key(&method.key);
                    names.insert(name.clone(), format!("__ayy$private${binding_name}${name}"));
                }
                ClassMember::AutoAccessor(accessor) => {
                    if let Key::Private(private_name) = &accessor.key {
                        let name = self.private_name_key(private_name);
                        names.insert(name.clone(), format!("__ayy$private${binding_name}${name}"));
                    }
                }
                _ => {}
            }
        }
        names
    }

    pub(crate) fn class_private_brand_map(
        &self,
        class: &Class,
        instance_private_brand_binding: Option<&str>,
    ) -> HashMap<String, String> {
        let Some(instance_private_brand_binding) = instance_private_brand_binding else {
            return HashMap::new();
        };
        let mut names = HashMap::new();
        for member in &class.body {
            match member {
                ClassMember::PrivateProp(property) => {
                    let name = self.private_name_key(&property.key);
                    names.insert(name, instance_private_brand_binding.to_string());
                }
                ClassMember::PrivateMethod(method) => {
                    let name = self.private_name_key(&method.key);
                    names.insert(name, instance_private_brand_binding.to_string());
                }
                ClassMember::AutoAccessor(accessor) => {
                    if let Key::Private(private_name) = &accessor.key {
                        let name = self.private_name_key(private_name);
                        names.insert(name, instance_private_brand_binding.to_string());
                    }
                }
                _ => {}
            }
        }
        names
    }

    pub(crate) fn current_strict_mode(&self) -> bool {
        self.strict_modes.last().copied().unwrap_or(false)
    }

    pub(crate) fn function_strict_mode(&self, function: &Function) -> bool {
        self.current_strict_mode() || function_has_use_strict_directive(function)
    }

    pub(crate) fn arrow_strict_mode(&self, arrow_expression: &ArrowExpr) -> bool {
        self.current_strict_mode()
            || match &*arrow_expression.body {
                BlockStmtOrExpr::BlockStmt(block) => script_has_use_strict_directive(&block.stmts),
                BlockStmtOrExpr::Expr(_) => false,
            }
    }

    pub(crate) fn function_has_mapped_arguments(&self, function: &Function) -> bool {
        !self.function_strict_mode(function) && function_has_simple_parameter_list(function)
    }
}
