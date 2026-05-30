use super::super::*;

impl ModuleLinker {
    pub(super) fn dynamic_import_specifier_sources_for_script(
        &self,
        script: &swc_ecma_ast::Script,
        source_text: &str,
    ) -> Vec<String> {
        let mut sources = collect_literal_dynamic_import_specifiers_in_statements(&script.body);
        for source in collect_literal_dynamic_import_specifiers_in_source_comments(source_text) {
            if !sources.contains(&source) {
                sources.push(source);
            }
        }
        sources
    }

    pub(super) fn dynamic_import_specifier_sources_for_module(
        &self,
        module: &Module,
        source_text: &str,
    ) -> Vec<String> {
        let mut sources = collect_literal_dynamic_import_specifiers(module);
        for source in collect_literal_dynamic_import_specifiers_in_source_comments(source_text) {
            if !sources.contains(&source) {
                sources.push(source);
            }
        }
        sources
    }

    pub(super) fn dynamic_import_specifier_index_lookup(
        &self,
        module_path: &Path,
        sources: &[String],
    ) -> BTreeMap<String, usize> {
        sources
            .iter()
            .filter_map(|source| {
                let resolved = resolve_module_specifier(module_path, source).ok()?;
                let module_index = self.module_indices.get(&resolved).copied()?;
                Some((source.clone(), module_index))
            })
            .collect()
    }

    pub(crate) fn bundle_entry(&mut self, path: &Path) -> Result<Program> {
        let entry_index = self.load_module(path)?;
        let (load_order, deferred_async_modules) = self.compute_static_load_order(entry_index);
        self.load_order = load_order;
        self.deferred_async_modules = deferred_async_modules;
        self.validate_loaded_module_export_resolutions()?;
        let statements = self.bundle_statements(entry_index)?;
        Ok(self.lowerer.finish_program(statements, true))
    }

    pub(crate) fn bundle_script_entry(&mut self, path: &Path) -> Result<Program> {
        let (script, lowered_source) = parse_script_file(path)?;
        let dynamic_import_sources =
            self.dynamic_import_specifier_sources_for_script(&script, &lowered_source);
        for source in &dynamic_import_sources {
            if let Ok(dependency_path) = resolve_module_specifier(path, source) {
                self.load_dynamic_module_with_type(&dependency_path, None)?;
            }
        }

        self.lowerer.source_text = Some(lowered_source);
        self.lowerer.current_module_path = Some(normalize_module_path(path)?);
        self.lowerer.module_index_lookup = self.module_indices.clone();
        self.lowerer.dynamic_import_specifier_lookup =
            self.dynamic_import_specifier_index_lookup(path, &dynamic_import_sources);
        let strict = script_has_use_strict_directive(&script.body);
        self.lowerer.strict_modes.push(strict);
        self.lowerer.module_mode = false;

        let mut statements = self.module_registry_statements();
        let scope_bindings = collect_direct_statement_lexical_bindings(&script.body)?;
        self.lowerer.push_binding_scope(scope_bindings);
        let lowered = self
            .lowerer
            .lower_top_level_statements(script.body.iter(), &mut statements);
        self.lowerer.pop_binding_scope();
        lowered?;

        self.lowerer.strict_modes.pop();
        self.lowerer.module_mode = false;
        self.lowerer.source_text = None;
        self.lowerer.current_module_path = None;
        self.lowerer.module_index_lookup.clear();
        self.lowerer.dynamic_import_specifier_lookup.clear();

        Ok(self.lowerer.finish_program(statements, strict))
    }
}
