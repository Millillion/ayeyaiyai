use super::function_registry::PreparedFunctionParameterBindings;
use crate::backend::direct_wasm::{
    ArgumentsValueBinding, ArrayValueBinding, DirectWasmCompiler, Expression, FunctionDeclaration,
    GlobalArgumentsValueQueryAccess, GlobalArrayValueQueryAccess, GlobalBindingEnvironment,
    GlobalBindingIndexQueryAccess, GlobalBindingKindQueryAccess, GlobalBindingPresenceQueryAccess,
    GlobalFunctionBindingQueryAccess, GlobalIdentifierValueQueryAccess,
    GlobalImplicitBindingQueryAccess, GlobalMemberAccessorMutationAccess,
    GlobalMemberAccessorQueryAccess, GlobalMemberBindingClearAccess,
    GlobalMemberCaptureMutationAccess, GlobalMemberCaptureQueryAccess,
    GlobalMemberFunctionMutationAccess, GlobalMemberFunctionQueryAccess,
    GlobalObjectValueQueryAccess, GlobalRuntimePrototypeQueryAccess, GlobalStaticSemanticsSnapshot,
    GlobalValueBindingQueryAccess, ImplicitGlobalBinding, JS_UNDEFINED_TAG, LocalFunctionBinding,
    MemberFunctionBindingKey, NEXT_PRIVATE_BRAND_GLOBAL_INDEX, ObjectValueBinding,
    PreparedModuleLayout, Program, ReturnedMemberFunctionBinding, StaticValueKind, UserFunction,
};
use std::collections::{BTreeMap, HashMap, HashSet};

mod lifecycle;
mod mutations;
mod queries;
mod registry;
mod runtime_prototypes;
