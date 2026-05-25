use std::{ffi::OsStr, fs};

use serde_json::Value as JsonValue;

use super::super::*;

impl ModuleLinker {
    pub(crate) fn ensure_module_slot(&mut self, path: &Path) -> Result<usize> {
        let resolved = normalize_module_path(path)?;
        if let Some(index) = self.module_indices.get(&resolved).copied() {
            return Ok(index);
        }

        let module_index = self.modules.len();
        self.module_indices.insert(resolved.clone(), module_index);
        self.modules.push(LinkedModule {
            path: resolved.clone(),
            state: ModuleState::Reserved,
            namespace_name: format!("__ayy_module_namespace_{module_index}"),
            init_name: format!("__ayy_module_init_{module_index}"),
            promise_name: format!("__ayy_module_promise_{module_index}"),
            init_async: false,
            dependency_params: Vec::new(),
            export_names: Vec::new(),
            export_resolutions: BTreeMap::new(),
            ambiguous_export_names: HashSet::new(),
        });

        Ok(module_index)
    }

    pub(crate) fn load_module(&mut self, path: &Path) -> Result<usize> {
        let module_index = self.ensure_module_slot(path)?;
        if self.modules[module_index].state != ModuleState::Reserved {
            return Ok(module_index);
        }

        let resolved = self.modules[module_index].path.clone();
        if resolved.extension() == Some(OsStr::new("json")) {
            self.load_json_module(module_index, &resolved)?;
            return Ok(module_index);
        }

        let (module, source_text) = parse_module_file(&resolved)?;
        self.modules[module_index].state = ModuleState::Lowering;
        self.predeclare_module_export_resolutions(module_index, &module, &resolved)?;
        self.lower_module(module_index, &module, source_text)?;
        self.modules[module_index].state = ModuleState::Lowered;

        Ok(module_index)
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

        let mut init_body = self.build_module_namespace_prelude(&exports_param);
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
        self.modules[module_index].dependency_params = Vec::new();
        self.modules[module_index].export_names = vec!["default".to_string()];
        self.modules[module_index].export_resolutions = export_resolutions;
        self.modules[module_index].ambiguous_export_names = HashSet::new();
        self.modules[module_index].state = ModuleState::Lowered;

        Ok(())
    }

    pub(crate) fn compute_static_load_order(&self, entry_index: usize) -> Vec<usize> {
        fn visit(
            linker: &ModuleLinker,
            module_index: usize,
            visited: &mut HashSet<usize>,
            order: &mut Vec<usize>,
        ) {
            if !visited.insert(module_index) {
                return;
            }

            for dependency in &linker.modules[module_index].dependency_params {
                if dependency.module_index != module_index {
                    visit(linker, dependency.module_index, visited, order);
                }
            }

            order.push(module_index);
        }

        let mut order = Vec::new();
        visit(self, entry_index, &mut HashSet::new(), &mut order);
        order
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
