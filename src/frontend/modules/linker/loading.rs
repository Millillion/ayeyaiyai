use std::{ffi::OsStr, fs};

use serde_json::Value as JsonValue;

use super::super::*;

impl ModuleLinker {
    fn reserve_module_slot(&mut self, resolved: PathBuf) -> usize {
        let module_index = self.modules.len();
        self.modules.push(LinkedModule {
            path: resolved,
            state: ModuleState::Reserved,
            load_error: None,
            namespace_name: format!("__ayy_module_namespace_{module_index}"),
            deferred_namespace_name: format!("__ayy_module_deferred_namespace_{module_index}"),
            status_name: format!("__ayy_module_status_{module_index}"),
            error_name: format!("__ayy_module_error_{module_index}"),
            init_name: format!("__ayy_module_init_{module_index}"),
            promise_name: format!("__ayy_module_promise_{module_index}"),
            async_continuation_names: Vec::new(),
            init_async: false,
            dependency_params: Vec::new(),
            export_names: Vec::new(),
            export_resolutions: BTreeMap::new(),
            star_export_module_indices: Vec::new(),
            ambiguous_export_names: HashSet::new(),
            pending_import_resolutions: Vec::new(),
        });
        module_index
    }

    pub(crate) fn ensure_module_slot(&mut self, path: &Path) -> Result<usize> {
        let resolved = normalize_module_path(path)?;
        if let Some(index) = self.module_indices.get(&resolved).copied() {
            return Ok(index);
        }

        let module_index = self.reserve_module_slot(resolved.clone());
        self.module_indices.insert(resolved.clone(), module_index);

        Ok(module_index)
    }

    fn ensure_text_module_slot(&mut self, path: &Path) -> Result<usize> {
        let resolved = normalize_module_path(path)?;
        if let Some(index) = self.text_module_indices.get(&resolved).copied() {
            return Ok(index);
        }

        let module_index = self.reserve_module_slot(resolved.clone());
        self.text_module_indices.insert(resolved, module_index);
        Ok(module_index)
    }

    fn ensure_bytes_module_slot(&mut self, path: &Path) -> Result<usize> {
        let resolved = normalize_module_path(path)?;
        if let Some(index) = self.bytes_module_indices.get(&resolved).copied() {
            return Ok(index);
        }

        let module_index = self.reserve_module_slot(resolved.clone());
        self.bytes_module_indices.insert(resolved, module_index);
        Ok(module_index)
    }

    pub(crate) fn load_module(&mut self, path: &Path) -> Result<usize> {
        self.load_module_with_type(path, None)
    }

    pub(crate) fn load_module_with_type(
        &mut self,
        path: &Path,
        import_type: Option<&str>,
    ) -> Result<usize> {
        let module_index = match import_type {
            Some("text") => self.ensure_text_module_slot(path)?,
            Some("bytes") => self.ensure_bytes_module_slot(path)?,
            _ => self.ensure_module_slot(path)?,
        };
        if self.modules[module_index].state == ModuleState::Failed {
            bail!(
                "{}",
                self.modules[module_index]
                    .load_error
                    .as_deref()
                    .unwrap_or("module failed to load")
            );
        }
        if self.modules[module_index].state != ModuleState::Reserved {
            return Ok(module_index);
        }

        let resolved = self.modules[module_index].path.clone();
        let load_result = if import_type == Some("text") {
            self.load_text_module(module_index, &resolved)
        } else if import_type == Some("bytes") {
            self.load_bytes_module(module_index, &resolved)
        } else if import_type == Some("json") || resolved.extension() == Some(OsStr::new("json")) {
            self.load_json_module(module_index, &resolved)
        } else {
            (|| -> Result<()> {
                let (module, source_text) = parse_module_file(&resolved)?;
                self.modules[module_index].state = ModuleState::Lowering;
                self.predeclare_module_export_resolutions(module_index, &module, &resolved)?;
                self.lower_module(module_index, &module, source_text)?;
                self.modules[module_index].state = ModuleState::Lowered;
                Ok(())
            })()
        };

        if let Err(error) = load_result {
            let message = format!("{error:#}");
            self.lower_failed_module(module_index, message.clone());
            bail!("{message}");
        }

        Ok(module_index)
    }

    pub(crate) fn load_dynamic_module_with_type(
        &mut self,
        path: &Path,
        import_type: Option<&str>,
    ) -> Result<usize> {
        let module_index = match import_type {
            Some("text") => self.ensure_text_module_slot(path)?,
            Some("bytes") => self.ensure_bytes_module_slot(path)?,
            _ => self.ensure_module_slot(path)?,
        };
        if self.load_module_with_type(path, import_type).is_err() {
            return Ok(module_index);
        }
        Ok(module_index)
    }

    fn lower_failed_module(&mut self, module_index: usize, message: String) {
        if self.modules[module_index].state == ModuleState::Failed {
            return;
        }

        let exports_param = "exports".to_string();
        self.lowerer.functions.push(FunctionDeclaration {
            name: self.modules[module_index].init_name.clone(),
            top_level_binding: None,
            params: vec![Parameter {
                name: exports_param,
                default: None,
                rest: false,
            }],
            body: vec![Statement::Throw(Expression::New {
                callee: Box::new(Expression::Identifier("TypeError".to_string())),
                arguments: vec![CallArgument::Expression(Expression::String(
                    message.clone(),
                ))],
            })],
            register_global: false,
            kind: FunctionKind::Ordinary,
            self_binding: None,
            mapped_arguments: false,
            strict: true,
            lexical_this: false,
            constructible: true,
            derived_constructor: false,
            direct_eval_in_class_field_initializer: false,
            length: 1,
            synthetic_capture_bindings: Vec::new(),
            immutable_class_bindings: Vec::new(),
            private_brand_binding: None,
        });

        self.modules[module_index].state = ModuleState::Failed;
        self.modules[module_index].load_error = Some(message);
        self.modules[module_index].async_continuation_names.clear();
        self.modules[module_index].init_async = false;
        self.modules[module_index].dependency_params.clear();
        self.modules[module_index].export_names.clear();
        self.modules[module_index].export_resolutions.clear();
        self.modules[module_index]
            .star_export_module_indices
            .clear();
        self.modules[module_index].ambiguous_export_names.clear();
        self.modules[module_index]
            .pending_import_resolutions
            .clear();
    }

    fn load_bytes_module(&mut self, module_index: usize, path: &Path) -> Result<()> {
        self.modules[module_index].state = ModuleState::Lowering;

        let bytes = fs::read(path)
            .with_context(|| format!("failed to read bytes module `{}`", path.display()))?;
        let default_binding = format!("__ayy_bytes_default_{module_index}");
        let exports_param = "exports".to_string();

        let mut export_resolutions = BTreeMap::new();
        export_resolutions.insert(
            "default".to_string(),
            ExportResolution::Binding {
                module_index,
                binding_name: default_binding.clone(),
                local: true,
            },
        );

        let mut export_expressions = BTreeMap::new();
        export_expressions.insert(
            "default".to_string(),
            Expression::Identifier(default_binding.clone()),
        );

        let byte_array = Expression::Array(
            bytes
                .into_iter()
                .map(|byte| ArrayElement::Expression(Expression::Number(byte as f64)))
                .collect(),
        );
        let deferred_exports = self.modules[module_index].deferred_namespace_name.clone();
        let mut init_body = self.build_module_namespace_prelude(&exports_param);
        init_body.extend(
            self.build_module_namespace_prelude_with_tag(&deferred_exports, "Deferred Module"),
        );
        init_body.push(Statement::Let {
            name: default_binding,
            mutable: false,
            value: Expression::New {
                callee: Box::new(Expression::Identifier("Uint8Array".to_string())),
                arguments: vec![CallArgument::Expression(byte_array)],
            },
        });
        init_body.extend(self.build_export_getter_statements(
            module_index,
            &exports_param,
            &export_expressions,
            &BTreeMap::new(),
            &BTreeMap::new(),
            &HashMap::new(),
        )?);
        init_body.extend(self.build_export_getter_statements(
            module_index,
            &deferred_exports,
            &export_expressions,
            &BTreeMap::new(),
            &BTreeMap::new(),
            &HashMap::new(),
        )?);
        self.mark_module_init_body_status(module_index, &mut init_body);
        let (init_body, init_async) = asyncify_statements(init_body);

        self.lowerer.functions.push(FunctionDeclaration {
            name: self.modules[module_index].init_name.clone(),
            top_level_binding: None,
            params: vec![Parameter {
                name: exports_param,
                default: None,
                rest: false,
            }],
            body: init_body,
            register_global: false,
            kind: FunctionKind::from_flags(false, init_async),
            self_binding: None,
            mapped_arguments: false,
            strict: true,
            lexical_this: false,
            constructible: true,
            derived_constructor: false,
            direct_eval_in_class_field_initializer: false,
            length: 1,
            synthetic_capture_bindings: Vec::new(),
            immutable_class_bindings: Vec::new(),
            private_brand_binding: None,
        });

        self.modules[module_index].init_async = init_async;
        self.modules[module_index].async_continuation_names = Vec::new();
        self.modules[module_index].dependency_params = Vec::new();
        self.modules[module_index].export_names = vec!["default".to_string()];
        self.modules[module_index].export_resolutions = export_resolutions;
        self.modules[module_index].star_export_module_indices = Vec::new();
        self.modules[module_index].ambiguous_export_names = HashSet::new();
        self.modules[module_index]
            .pending_import_resolutions
            .clear();
        self.modules[module_index].state = ModuleState::Lowered;

        Ok(())
    }

    fn load_text_module(&mut self, module_index: usize, path: &Path) -> Result<()> {
        self.modules[module_index].state = ModuleState::Lowering;

        let source = fs::read_to_string(path)
            .with_context(|| format!("failed to read text module `{}`", path.display()))?;
        let default_binding = format!("__ayy_text_default_{module_index}");
        let exports_param = "exports".to_string();

        let mut export_resolutions = BTreeMap::new();
        export_resolutions.insert(
            "default".to_string(),
            ExportResolution::Binding {
                module_index,
                binding_name: default_binding.clone(),
                local: true,
            },
        );

        let mut export_expressions = BTreeMap::new();
        export_expressions.insert(
            "default".to_string(),
            Expression::Identifier(default_binding.clone()),
        );

        let deferred_exports = self.modules[module_index].deferred_namespace_name.clone();
        let mut init_body = self.build_module_namespace_prelude(&exports_param);
        init_body.extend(
            self.build_module_namespace_prelude_with_tag(&deferred_exports, "Deferred Module"),
        );
        init_body.push(Statement::Let {
            name: default_binding,
            mutable: false,
            value: Expression::String(source),
        });
        init_body.extend(self.build_export_getter_statements(
            module_index,
            &exports_param,
            &export_expressions,
            &BTreeMap::new(),
            &BTreeMap::new(),
            &HashMap::new(),
        )?);
        init_body.extend(self.build_export_getter_statements(
            module_index,
            &deferred_exports,
            &export_expressions,
            &BTreeMap::new(),
            &BTreeMap::new(),
            &HashMap::new(),
        )?);
        self.mark_module_init_body_status(module_index, &mut init_body);
        let (init_body, init_async) = asyncify_statements(init_body);

        self.lowerer.functions.push(FunctionDeclaration {
            name: self.modules[module_index].init_name.clone(),
            top_level_binding: None,
            params: vec![Parameter {
                name: exports_param,
                default: None,
                rest: false,
            }],
            body: init_body,
            register_global: false,
            kind: FunctionKind::from_flags(false, init_async),
            self_binding: None,
            mapped_arguments: false,
            strict: true,
            lexical_this: false,
            constructible: true,
            derived_constructor: false,
            direct_eval_in_class_field_initializer: false,
            length: 1,
            synthetic_capture_bindings: Vec::new(),
            immutable_class_bindings: Vec::new(),
            private_brand_binding: None,
        });

        self.modules[module_index].init_async = init_async;
        self.modules[module_index].async_continuation_names = Vec::new();
        self.modules[module_index].dependency_params = Vec::new();
        self.modules[module_index].export_names = vec!["default".to_string()];
        self.modules[module_index].export_resolutions = export_resolutions;
        self.modules[module_index].star_export_module_indices = Vec::new();
        self.modules[module_index].ambiguous_export_names = HashSet::new();
        self.modules[module_index]
            .pending_import_resolutions
            .clear();
        self.modules[module_index].state = ModuleState::Lowered;

        Ok(())
    }

    fn load_json_module(&mut self, module_index: usize, path: &Path) -> Result<()> {
        self.modules[module_index].state = ModuleState::Lowering;

        let source = fs::read_to_string(path)
            .with_context(|| format!("failed to read JSON module `{}`", path.display()))?;
        let json = serde_json::from_str::<JsonValue>(&source)
            .with_context(|| format!("failed to parse JSON module `{}`", path.display()))?;
        let default_binding = format!("__ayy_json_default_{module_index}");
        let text_binding = format!("__ayy_text_default_{module_index}");
        let exports_param = "exports".to_string();

        let mut export_resolutions = BTreeMap::new();
        export_resolutions.insert(
            "default".to_string(),
            ExportResolution::Binding {
                module_index,
                binding_name: default_binding.clone(),
                local: true,
            },
        );

        let mut export_expressions = BTreeMap::new();
        export_expressions.insert(
            "default".to_string(),
            Expression::Identifier(default_binding.clone()),
        );

        let deferred_exports = self.modules[module_index].deferred_namespace_name.clone();
        let mut init_body = self.build_module_namespace_prelude(&exports_param);
        init_body.extend(
            self.build_module_namespace_prelude_with_tag(&deferred_exports, "Deferred Module"),
        );
        init_body.push(Statement::Let {
            name: text_binding,
            mutable: false,
            value: Expression::String(source),
        });
        init_body.push(Statement::Let {
            name: default_binding,
            mutable: false,
            value: json_value_to_expression(&json)?,
        });
        init_body.extend(self.build_export_getter_statements(
            module_index,
            &exports_param,
            &export_expressions,
            &BTreeMap::new(),
            &BTreeMap::new(),
            &HashMap::new(),
        )?);
        init_body.extend(self.build_export_getter_statements(
            module_index,
            &deferred_exports,
            &export_expressions,
            &BTreeMap::new(),
            &BTreeMap::new(),
            &HashMap::new(),
        )?);
        self.mark_module_init_body_status(module_index, &mut init_body);
        let (init_body, init_async) = asyncify_statements(init_body);

        self.lowerer.functions.push(FunctionDeclaration {
            name: self.modules[module_index].init_name.clone(),
            top_level_binding: None,
            params: vec![Parameter {
                name: exports_param,
                default: None,
                rest: false,
            }],
            body: init_body,
            register_global: false,
            kind: FunctionKind::from_flags(false, init_async),
            self_binding: None,
            mapped_arguments: false,
            strict: true,
            lexical_this: false,
            constructible: true,
            derived_constructor: false,
            direct_eval_in_class_field_initializer: false,
            length: 1,
            synthetic_capture_bindings: Vec::new(),
            immutable_class_bindings: Vec::new(),
            private_brand_binding: None,
        });

        self.modules[module_index].init_async = init_async;
        self.modules[module_index].async_continuation_names = Vec::new();
        self.modules[module_index].dependency_params = Vec::new();
        self.modules[module_index].export_names = vec!["default".to_string()];
        self.modules[module_index].export_resolutions = export_resolutions;
        self.modules[module_index].star_export_module_indices = Vec::new();
        self.modules[module_index].ambiguous_export_names = HashSet::new();
        self.modules[module_index]
            .pending_import_resolutions
            .clear();
        self.modules[module_index].state = ModuleState::Lowered;

        Ok(())
    }

    pub(crate) fn compute_static_load_order(
        &self,
        entry_index: usize,
    ) -> (Vec<usize>, HashSet<usize>) {
        fn gather_async_transitive_dependencies(
            linker: &ModuleLinker,
            module_index: usize,
            seen: &mut HashSet<usize>,
            result: &mut Vec<usize>,
        ) {
            if !seen.insert(module_index) {
                return;
            }

            if linker.modules[module_index].init_async {
                if !result.contains(&module_index) {
                    result.push(module_index);
                }
                return;
            }

            for dependency in &linker.modules[module_index].dependency_params {
                if dependency.module_index != module_index {
                    gather_async_transitive_dependencies(
                        linker,
                        dependency.module_index,
                        seen,
                        result,
                    );
                }
            }
        }

        fn visit(
            linker: &ModuleLinker,
            module_index: usize,
            deferred_async_context: bool,
            visited: &mut HashSet<usize>,
            order: &mut Vec<usize>,
            deferred_async_modules: &mut HashSet<usize>,
        ) {
            if !visited.insert(module_index) {
                return;
            }

            for dependency in &linker.modules[module_index].dependency_params {
                if dependency.deferred {
                    let mut async_dependencies = Vec::new();
                    gather_async_transitive_dependencies(
                        linker,
                        dependency.module_index,
                        &mut HashSet::new(),
                        &mut async_dependencies,
                    );
                    for async_dependency in async_dependencies {
                        if async_dependency != module_index {
                            visit(
                                linker,
                                async_dependency,
                                true,
                                visited,
                                order,
                                deferred_async_modules,
                            );
                        }
                    }
                } else if dependency.eager && dependency.module_index != module_index {
                    visit(
                        linker,
                        dependency.module_index,
                        deferred_async_context,
                        visited,
                        order,
                        deferred_async_modules,
                    );
                }
            }

            if deferred_async_context && linker.modules[module_index].init_async {
                deferred_async_modules.insert(module_index);
            }
            order.push(module_index);
        }

        let mut order = Vec::new();
        let mut deferred_async_modules = HashSet::new();
        visit(
            self,
            entry_index,
            false,
            &mut HashSet::new(),
            &mut order,
            &mut deferred_async_modules,
        );
        (order, deferred_async_modules)
    }
}

fn json_value_to_expression(value: &JsonValue) -> Result<Expression> {
    Ok(match value {
        JsonValue::Null => Expression::Null,
        JsonValue::Bool(value) => Expression::Bool(*value),
        JsonValue::Number(value) => {
            let value = value
                .as_f64()
                .context("JSON module number is outside the supported f64 range")?;
            Expression::Number(value)
        }
        JsonValue::String(value) => Expression::String(value.clone()),
        JsonValue::Array(values) => Expression::Array(
            values
                .iter()
                .map(json_value_to_expression)
                .map(|value| value.map(ArrayElement::Expression))
                .collect::<Result<Vec<_>>>()?,
        ),
        JsonValue::Object(entries) => Expression::Object(
            entries
                .iter()
                .map(|(key, value)| {
                    Ok(ObjectEntry::Data {
                        key: Expression::String(key.clone()),
                        value: json_value_to_expression(value)?,
                    })
                })
                .collect::<Result<Vec<_>>>()?,
        ),
    })
}
