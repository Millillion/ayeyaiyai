use super::*;

struct PreparedIdentifierValueStore {
    canonical_value_expression: Expression,
    tracked_value_expression: Expression,
    descriptor_binding_expression: Expression,
    tracked_object_expression: Expression,
    call_source_snapshot_expression: Option<Expression>,
    prototype_source_snapshot_expression: Option<Expression>,
    function_binding_expression: Expression,
    function_binding: Option<LocalFunctionBinding>,
    object_binding_expression: Expression,
    object_binding: Option<ObjectValueBinding>,
    kind: Option<StaticValueKind>,
    static_string_value: Option<String>,
    exact_static_number: Option<f64>,
    array_binding: Option<ArrayValueBinding>,
    module_assignment_expression: Expression,
    resolved_local_binding: Option<(String, u32)>,
    returned_descriptor_binding: Option<PropertyDescriptorBinding>,
    runtime_value_override: Option<Expression>,
    opaque_runtime_value: bool,
}

fn expression_is_object_create_null_call(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::Call { callee, arguments }
            if matches!(
                callee.as_ref(),
                Expression::Member { object, property }
                    if matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
                        && matches!(property.as_ref(), Expression::String(name) if name == "create")
            )
                && matches!(arguments.as_slice(), [CallArgument::Expression(Expression::Null)])
    )
}

mod context;
mod store_paths;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn emit_store_identifier_value_local(
        &mut self,
        name: &str,
        value_expression: &Expression,
        value_local: u32,
    ) -> DirectResult<()> {
        if self.assignment_targets_immutable_class_binding(name) {
            self.emit_named_error_throw("TypeError")?;
            return Ok(());
        }
        if self.assignment_targets_immutable_function_self_binding(name) {
            if self.state.speculation.execution_context.strict_mode {
                self.emit_named_error_throw("TypeError")?;
            }
            return Ok(());
        }
        self.state
            .runtime
            .locals
            .deleted_builtin_identifiers
            .remove(name);
        let prepared = self.prepare_identifier_value_store(name, value_expression);
        let result = self.store_prepared_identifier_value_local(name, value_local, prepared);
        if result.is_ok() {
            self.state
                .emission
                .emitted_value_bindings
                .insert(name.to_string());
        }
        result
    }

    pub(in crate::backend::direct_wasm) fn emit_initialize_identifier_value_local(
        &mut self,
        name: &str,
        value_expression: &Expression,
        value_local: u32,
    ) -> DirectResult<()> {
        self.state
            .runtime
            .locals
            .deleted_builtin_identifiers
            .remove(name);
        let prepared = self.prepare_identifier_value_store(name, value_expression);
        let result = self.store_prepared_identifier_value_local_for_initialization(
            name,
            value_local,
            prepared,
        );
        if result.is_ok() {
            self.state
                .emission
                .emitted_value_bindings
                .insert(name.to_string());
        }
        result
    }
}
