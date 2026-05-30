use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail, ensure};
use swc_ecma_ast::*;

use crate::ir::hir::{
    ArrayElement, CallArgument, Expression, FunctionDeclaration, FunctionKind, ObjectEntry,
    Parameter, Program, Statement,
};

use super::{
    early_errors::{
        collect_module_declared_names, collect_var_decl_bound_names,
        ensure_module_lexical_names_are_unique, script_has_use_strict_directive,
        validate_import_attributes,
    },
    lowering::{
        Lowerer, asyncify_statements, collect_direct_statement_lexical_bindings,
        data_property_descriptor, define_property_statement,
    },
    parse::{parse_module_file, parse_script_file},
};

mod dynamic_imports;
mod emit;
mod export_resolution;
mod import_rewriter;
mod linker;
pub(crate) mod resolution;

use self::{
    dynamic_imports::{
        collect_literal_dynamic_import_specifiers,
        collect_literal_dynamic_import_specifiers_in_source_comments,
        collect_literal_dynamic_import_specifiers_in_statements,
    },
    export_resolution::{import_attribute_type, module_export_name_string},
    import_rewriter::rewrite_module_import_bindings_in_function,
    resolution::{normalize_module_path, resolve_module_specifier},
};

pub fn bundle_module_entry(path: &Path) -> Result<Program> {
    ModuleLinker::default().bundle_entry(path)
}

pub fn bundle_script_entry(path: &Path) -> Result<Program> {
    ModuleLinker::default().bundle_script_entry(path)
}

#[derive(Default)]
struct ModuleLinker {
    lowerer: Lowerer,
    modules: Vec<LinkedModule>,
    module_indices: HashMap<PathBuf, usize>,
    text_module_indices: HashMap<PathBuf, usize>,
    bytes_module_indices: HashMap<PathBuf, usize>,
    load_order: Vec<usize>,
    deferred_async_modules: HashSet<usize>,
}

#[derive(Clone)]
struct LinkedModule {
    path: PathBuf,
    state: ModuleState,
    load_error: Option<String>,
    namespace_name: String,
    deferred_namespace_name: String,
    status_name: String,
    error_name: String,
    init_name: String,
    promise_name: String,
    async_continuation_names: Vec<String>,
    init_async: bool,
    dependency_params: Vec<ModuleDependencyParam>,
    export_names: Vec<String>,
    export_resolutions: BTreeMap<String, ExportResolution>,
    star_export_module_indices: Vec<usize>,
    ambiguous_export_names: HashSet<String>,
    pending_import_resolutions: Vec<(usize, String)>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ModuleState {
    Reserved,
    Lowering,
    Lowered,
    Failed,
}

#[derive(Clone)]
struct ModuleDependencyParam {
    module_index: usize,
    param_name: String,
    eager: bool,
    deferred: bool,
}

#[derive(Clone)]
enum ImportBinding {
    Namespace {
        module_index: usize,
        namespace_param: String,
        deferred: bool,
    },
    Named {
        module_index: usize,
        namespace_param: String,
        export_name: String,
        self_local_binding: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ExportResolution {
    Binding {
        module_index: usize,
        binding_name: String,
        local: bool,
    },
    Namespace {
        module_index: usize,
    },
}
