use super::*;

thread_local! {
    static ACTIVE_RUNTIME_SHADOW_FALLBACKS: std::cell::RefCell<std::collections::HashSet<String>> =
        std::cell::RefCell::new(std::collections::HashSet::new());
    static ACTIVE_RUNTIME_SHADOW_PREFIX_CACHE: std::cell::RefCell<ActiveRuntimeShadowPrefixCache> =
        std::cell::RefCell::new(ActiveRuntimeShadowPrefixCache::new());
}

struct RuntimeShadowFallbackGuard {
    key: String,
    _memo: crate::backend::direct_wasm::memo::ResolutionGuardScope,
}

struct ActiveRuntimeShadowPrefixCache {
    generation: u64,
    names: std::collections::HashMap<String, Vec<(u32, String)>>,
    exists: std::collections::HashMap<String, bool>,
    implicit_bindings: std::collections::HashMap<String, Vec<(String, ImplicitGlobalBinding)>>,
}

struct RuntimeMemberShadowAliasOwner {
    owner: String,
    guard: Option<RuntimeMemberShadowAliasGuard>,
}

struct RuntimeMemberShadowAliasGuard {
    parent_owner: String,
    parent_property: Expression,
    assigned_property: Expression,
    depth: usize,
}

impl ActiveRuntimeShadowPrefixCache {
    fn new() -> Self {
        Self {
            generation: 0,
            names: std::collections::HashMap::new(),
            exists: std::collections::HashMap::new(),
            implicit_bindings: std::collections::HashMap::new(),
        }
    }

    fn sync_generation(&mut self) {
        let generation = crate::backend::direct_wasm::memo::static_state_generation();
        if self.generation != generation {
            self.generation = generation;
            self.names.clear();
            self.exists.clear();
            self.implicit_bindings.clear();
        }
    }
}

impl RuntimeShadowFallbackGuard {
    fn enter(fallback_value: &Expression) -> Option<Self> {
        let key = format!("{fallback_value:?}");
        let inserted =
            ACTIVE_RUNTIME_SHADOW_FALLBACKS.with(|active| active.borrow_mut().insert(key.clone()));
        if !inserted {
            crate::backend::direct_wasm::memo::note_resolution_guard_block();
        }
        inserted.then_some(Self {
            key,
            _memo: crate::backend::direct_wasm::memo::ResolutionGuardScope::enter_class(17),
        })
    }
}

impl Drop for RuntimeShadowFallbackGuard {
    fn drop(&mut self) {
        ACTIVE_RUNTIME_SHADOW_FALLBACKS.with(|active| {
            active.borrow_mut().remove(&self.key);
        });
    }
}

const RUNTIME_SHADOW_OWNER_EXPRESSION_RECURSION_LIMIT: usize = 64;
const RUNTIME_MEMBER_SHADOW_NULL_TAIL_ALIAS_DEPTH_LIMIT: usize = 2;

thread_local! {
    static RUNTIME_SHADOW_OWNER_EXPRESSION_DEPTH: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
}

struct RuntimeShadowOwnerExpressionGuard {
    _memo: Option<crate::backend::direct_wasm::memo::ResolutionGuardScope>,
}

impl RuntimeShadowOwnerExpressionGuard {
    fn enter() -> Option<Self> {
        RUNTIME_SHADOW_OWNER_EXPRESSION_DEPTH.with(|depth| {
            let current = depth.get();
            if current >= RUNTIME_SHADOW_OWNER_EXPRESSION_RECURSION_LIMIT {
                crate::backend::direct_wasm::memo::note_resolution_guard_block();
                return None;
            }
            depth.set(current + 1);
            // Only mark the resolution context as non-canonical once the
            // recursion is deep enough to suggest cyclic value chasing;
            // shallow nesting is the common, fully deterministic case.
            let memo = (current >= 8)
                .then(|| crate::backend::direct_wasm::memo::ResolutionGuardScope::enter_class(26));
            Some(Self { _memo: memo })
        })
    }
}

impl Drop for RuntimeShadowOwnerExpressionGuard {
    fn drop(&mut self) {
        RUNTIME_SHADOW_OWNER_EXPRESSION_DEPTH
            .with(|depth| depth.set(depth.get().saturating_sub(1)));
    }
}

fn expression_may_evaluate_to_runtime_shadow_owner(expression: &Expression) -> bool {
    match expression {
        Expression::Array(_)
        | Expression::Object(_)
        | Expression::Identifier(_)
        | Expression::This
        | Expression::Member { .. }
        | Expression::SuperMember { .. }
        | Expression::Call { .. }
        | Expression::SuperCall { .. }
        | Expression::New { .. }
        | Expression::Await(_)
        | Expression::EnumerateKeys(_)
        | Expression::GetIterator(_)
        | Expression::IteratorClose(_) => true,
        Expression::Assign { value, .. }
        | Expression::AssignMember { value, .. }
        | Expression::AssignSuperMember { value, .. } => {
            expression_may_evaluate_to_runtime_shadow_owner(value)
        }
        Expression::Binary {
            op: BinaryOp::LogicalAnd | BinaryOp::LogicalOr | BinaryOp::NullishCoalescing,
            left,
            right,
        } => {
            expression_may_evaluate_to_runtime_shadow_owner(left)
                || expression_may_evaluate_to_runtime_shadow_owner(right)
        }
        Expression::Conditional {
            then_expression,
            else_expression,
            ..
        } => {
            expression_may_evaluate_to_runtime_shadow_owner(then_expression)
                || expression_may_evaluate_to_runtime_shadow_owner(else_expression)
        }
        Expression::Sequence(expressions) => expressions
            .last()
            .is_some_and(expression_may_evaluate_to_runtime_shadow_owner),
        _ => false,
    }
}

impl<'a> FunctionCompiler<'a> {
    fn runtime_shadow_static_sync_owner_matches(object: &Expression, target_owner: &str) -> bool {
        matches!(object, Expression::This) && target_owner == "this"
            || matches!(object, Expression::Identifier(name) if name == target_owner)
    }

    fn runtime_shadow_static_sync_seed_value(&self, value: &Expression) -> Option<Expression> {
        if let Expression::Call { callee, arguments } = value {
            return self.resolve_effectful_call_return_metadata_value(callee, arguments);
        }
        self.resolve_static_primitive_expression_with_context(value, self.current_function_name())
    }

    fn rewrite_runtime_shadow_static_sync_current_binding_members(
        &self,
        expression: &Expression,
        target_owner: &str,
        object_binding: &ObjectValueBinding,
        depth: usize,
    ) -> Option<Expression> {
        const MAX_DEPTH: usize = 24;
        if depth >= MAX_DEPTH {
            return None;
        }

        match expression {
            Expression::Member { object, property }
                if Self::runtime_shadow_static_sync_owner_matches(object, target_owner) =>
            {
                let property = self.canonical_object_property_expression(property);
                if object_binding_lookup_descriptor(object_binding, &property)
                    .is_some_and(Self::property_descriptor_is_accessor)
                {
                    return None;
                }
                let value = object_binding_lookup_value(object_binding, &property)?;
                Some(
                    self.runtime_shadow_static_sync_seed_value(value)
                        .unwrap_or_else(|| value.clone()),
                )
            }
            Expression::Unary { op, expression } => {
                let expression = self.rewrite_runtime_shadow_static_sync_current_binding_members(
                    expression,
                    target_owner,
                    object_binding,
                    depth + 1,
                )?;
                Some(Expression::Unary {
                    op: *op,
                    expression: Box::new(expression),
                })
            }
            Expression::Binary { op, left, right } => {
                let left_rewrite = self.rewrite_runtime_shadow_static_sync_current_binding_members(
                    left,
                    target_owner,
                    object_binding,
                    depth + 1,
                );
                let right_rewrite = self
                    .rewrite_runtime_shadow_static_sync_current_binding_members(
                        right,
                        target_owner,
                        object_binding,
                        depth + 1,
                    );
                if left_rewrite.is_none() && right_rewrite.is_none() {
                    return None;
                }
                Some(Expression::Binary {
                    op: *op,
                    left: Box::new(left_rewrite.unwrap_or_else(|| left.as_ref().clone())),
                    right: Box::new(right_rewrite.unwrap_or_else(|| right.as_ref().clone())),
                })
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                let condition_rewrite = self
                    .rewrite_runtime_shadow_static_sync_current_binding_members(
                        condition,
                        target_owner,
                        object_binding,
                        depth + 1,
                    );
                let then_rewrite = self.rewrite_runtime_shadow_static_sync_current_binding_members(
                    then_expression,
                    target_owner,
                    object_binding,
                    depth + 1,
                );
                let else_rewrite = self.rewrite_runtime_shadow_static_sync_current_binding_members(
                    else_expression,
                    target_owner,
                    object_binding,
                    depth + 1,
                );
                if condition_rewrite.is_none() && then_rewrite.is_none() && else_rewrite.is_none() {
                    return None;
                }
                Some(Expression::Conditional {
                    condition: Box::new(
                        condition_rewrite.unwrap_or_else(|| condition.as_ref().clone()),
                    ),
                    then_expression: Box::new(
                        then_rewrite.unwrap_or_else(|| then_expression.as_ref().clone()),
                    ),
                    else_expression: Box::new(
                        else_rewrite.unwrap_or_else(|| else_expression.as_ref().clone()),
                    ),
                })
            }
            Expression::Sequence(expressions) => {
                let mut rewritten = None;
                let expressions = expressions
                    .iter()
                    .map(|expression| {
                        if let Some(rewrite) = self
                            .rewrite_runtime_shadow_static_sync_current_binding_members(
                                expression,
                                target_owner,
                                object_binding,
                                depth + 1,
                            )
                        {
                            rewritten = Some(());
                            rewrite
                        } else {
                            expression.clone()
                        }
                    })
                    .collect::<Vec<_>>();
                rewritten.map(|()| Expression::Sequence(expressions))
            }
            _ => None,
        }
    }

    fn resolve_runtime_shadow_static_sync_current_binding_primitive(
        &self,
        expression: &Expression,
        target_owner: &str,
        object_binding: &ObjectValueBinding,
    ) -> Option<Expression> {
        let rewritten = self.rewrite_runtime_shadow_static_sync_current_binding_members(
            expression,
            target_owner,
            object_binding,
            0,
        )?;
        self.resolve_static_primitive_expression_with_context(
            &rewritten,
            self.current_function_name(),
        )
    }

    fn runtime_shadow_value_may_have_member_shadows(&self, value: &Expression) -> bool {
        if !expression_may_evaluate_to_runtime_shadow_owner(value) {
            return false;
        }

        let resolved_call_value = match value {
            Expression::Call { callee, arguments } => {
                self.resolve_effectful_call_return_metadata_value(callee, arguments)
            }
            _ => None,
        };
        let kind = resolved_call_value
            .as_ref()
            .and_then(|value| self.infer_value_kind(value))
            .or_else(|| self.infer_value_kind(value));

        !matches!(
            kind,
            Some(
                StaticValueKind::Number
                    | StaticValueKind::Bool
                    | StaticValueKind::String
                    | StaticValueKind::BigInt
                    | StaticValueKind::Null
                    | StaticValueKind::Undefined
                    | StaticValueKind::Symbol
            )
        )
    }

    pub(in crate::backend::direct_wasm) fn reference_preserving_static_value_expression(
        &self,
        value: &Expression,
    ) -> Expression {
        if let Expression::Member { object, property } = value
            && let Some(live_binding_value) =
                self.resolve_module_namespace_live_binding_member_value(object, property)
        {
            let materialized = self.materialize_static_expression(&live_binding_value);
            if matches!(
                self.infer_value_kind(&materialized),
                Some(StaticValueKind::Object | StaticValueKind::Function)
            ) || self
                .resolve_object_binding_from_expression(&materialized)
                .is_some()
                || self
                    .resolve_array_binding_from_expression(&materialized)
                    .is_some()
            {
                return live_binding_value;
            }
        }
        let preserve_reference_alias =
            matches!(value, Expression::Identifier(_) | Expression::This)
                && (self
                    .runtime_array_binding_name_for_expression(value)
                    .is_some()
                    || self.resolve_array_binding_from_expression(value).is_some()
                    || self.resolve_object_binding_from_expression(value).is_some()
                    || self
                        .resolve_function_binding_from_expression(value)
                        .is_some());
        let preserve_private_brand =
            matches!(value, Expression::Identifier(name) if name.contains("__ayy_class_brand_"));
        if preserve_reference_alias || preserve_private_brand {
            value.clone()
        } else {
            self.materialize_static_expression(value)
        }
    }

    pub(in crate::backend::direct_wasm) fn emit_runtime_shadow_debug_print_local(
        &mut self,
        label: &str,
        value_local: u32,
    ) -> DirectResult<()> {
        let hidden_name =
            self.allocate_named_hidden_local("runtime_shadow_debug", StaticValueKind::Unknown);
        let hidden_local = self
            .state
            .runtime
            .locals
            .get(&hidden_name)
            .copied()
            .expect("fresh runtime shadow debug local must exist");
        self.push_local_get(value_local);
        self.push_local_set(hidden_local);
        self.emit_print(&[
            Expression::String(label.to_string()),
            Expression::Identifier(hidden_name),
        ])
    }

    pub(in crate::backend::direct_wasm) fn resolve_identifier_object_binding_fallback(
        &self,
        name: &str,
    ) -> Option<ObjectValueBinding> {
        self.current_function_name()
            .and_then(|function_name| {
                self.backend
                    .function_registry
                    .parameter_bindings_for(function_name)
                    .object_bindings
                    .get(name)
                    .cloned()
                    .flatten()
            })
            .or_else(|| self.global_object_binding(name).cloned())
    }

    fn runtime_object_property_shadow_fragment(text: &str) -> String {
        text.as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    pub(in crate::backend::direct_wasm) fn runtime_object_property_shadow_key(
        property: &Expression,
    ) -> String {
        if let Some(property_name) = static_property_name_from_expression(property) {
            return format!(
                "str__{}",
                Self::runtime_object_property_shadow_fragment(&property_name)
            );
        }

        format!(
            "expr__{}",
            Self::runtime_object_property_shadow_fragment(&format!("{property:?}"))
        )
    }

    fn canonical_runtime_shadow_property_expression(&self, property: &Expression) -> Expression {
        let materialized_property = self
            .resolve_property_key_expression(property)
            .unwrap_or_else(|| self.materialize_static_expression(property));
        if let Some(property_name) = static_property_name_from_expression(&materialized_property)
            .or_else(|| static_property_name_from_expression(property))
        {
            return Expression::String(property_name);
        }
        if let Some(symbol_identity) =
            self.resolve_symbol_identity_expression(&materialized_property)
        {
            return symbol_identity;
        }
        materialized_property
    }

    pub(in crate::backend::direct_wasm) fn runtime_object_member_shadow_owner_name(
        owner_name: &str,
        property: &Expression,
    ) -> String {
        format!(
            "__ayy_member_object__{}__{}",
            Self::runtime_object_property_shadow_fragment(owner_name),
            Self::runtime_object_property_shadow_key(property)
        )
    }

    pub(in crate::backend::direct_wasm) fn runtime_object_property_deleted_shadow_name(
        owner_name: &str,
        property: &Expression,
    ) -> String {
        format!(
            "__ayy_object_property_deleted__{owner_name}__{}",
            Self::runtime_object_property_shadow_key(property)
        )
    }

    fn runtime_object_dynamic_property_key_shadow_name(owner_name: &str) -> String {
        format!("__ayy_object_dynamic_property_key__{owner_name}")
    }

    fn runtime_object_dynamic_property_value_shadow_name(owner_name: &str) -> String {
        format!("__ayy_object_dynamic_property_value__{owner_name}")
    }

    pub(in crate::backend::direct_wasm) fn runtime_object_property_shadow_owner_has_bindings(
        &self,
        owner_name: &str,
    ) -> bool {
        let property_prefix = format!("__ayy_object_property__{owner_name}__");
        let deleted_prefix = format!("__ayy_object_property_deleted__{owner_name}__");
        self.active_runtime_object_shadow_prefix_exists(&property_prefix)
            || self.active_runtime_object_shadow_prefix_exists(&deleted_prefix)
    }

    pub(in crate::backend::direct_wasm) fn user_function_arguments_slot_object_shadow_owner_name(
        function_name: &str,
        index: u32,
    ) -> String {
        format!("__ayy_arguments_object_slot_{function_name}_{index}")
    }

    fn direct_arguments_slot_member_assignment_property(
        object: &Expression,
        property: &Expression,
        index: u32,
    ) -> Option<String> {
        let Expression::Member {
            object: base_object,
            property: base_property,
        } = object
        else {
            return None;
        };
        let Expression::Identifier(base_name) = base_object.as_ref() else {
            return None;
        };
        if scoped_binding_source_name(base_name).unwrap_or(base_name) != "arguments" {
            return None;
        }
        (argument_index_from_expression(base_property) == Some(index))
            .then(|| static_property_name_from_expression(property))
            .flatten()
    }

    fn collect_direct_arguments_slot_member_assignment_properties_from_expression(
        expression: &Expression,
        index: u32,
        properties: &mut BTreeSet<String>,
    ) {
        match expression {
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                if let Some(property_name) =
                    Self::direct_arguments_slot_member_assignment_property(object, property, index)
                {
                    properties.insert(property_name);
                }
                Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                    object, index, properties,
                );
                Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                    property, index, properties,
                );
                Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                    value, index, properties,
                );
            }
            Expression::Member { object, property } => {
                Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                    object, index, properties,
                );
                Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                    property, index, properties,
                );
            }
            Expression::Assign { value, .. }
            | Expression::AssignSuperMember { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => {
                Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                    value, index, properties,
                );
            }
            Expression::SuperMember { property } => {
                Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                    property, index, properties,
                );
            }
            Expression::Binary { left, right, .. } => {
                Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                    left, index, properties,
                );
                Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                    right, index, properties,
                );
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                    condition, index, properties,
                );
                Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                    then_expression,
                    index,
                    properties,
                );
                Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                    else_expression,
                    index,
                    properties,
                );
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                        expression, index, properties,
                    );
                }
            }
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                    callee, index, properties,
                );
                for argument in arguments {
                    Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                        argument.expression(),
                        index,
                        properties,
                    );
                }
            }
            Expression::Array(elements) => {
                for element in elements {
                    match element {
                        ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                            Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                                expression, index, properties,
                            );
                        }
                    }
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                                key, index, properties,
                            );
                            Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                                value, index, properties,
                            );
                        }
                        ObjectEntry::Getter { key, getter } => {
                            Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                                key, index, properties,
                            );
                            Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                                getter, index, properties,
                            );
                        }
                        ObjectEntry::Setter { key, setter } => {
                            Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                                key, index, properties,
                            );
                            Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                                setter, index, properties,
                            );
                        }
                        ObjectEntry::Spread(expression) => {
                            Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                                expression, index, properties,
                            );
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn direct_arguments_slot_assignment_properties(
        user_function: &UserFunction,
        index: u32,
    ) -> Vec<String> {
        let mut properties = BTreeSet::new();
        if let Some(summary) = user_function.inline_summary.as_ref() {
            for effect in &summary.effects {
                match effect {
                    InlineFunctionEffect::Assign { value, .. }
                    | InlineFunctionEffect::Expression(value) => {
                        Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                            value,
                            index,
                            &mut properties,
                        );
                    }
                    InlineFunctionEffect::Update { .. } => {}
                }
            }
            if let Some(return_value) = summary.return_value.as_ref() {
                Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                    return_value,
                    index,
                    &mut properties,
                );
            }
        }
        properties.into_iter().collect()
    }

    pub(in crate::backend::direct_wasm) fn predeclare_runtime_shadow_property(
        &mut self,
        owner_name: &str,
        property_name: &str,
    ) {
        let property = Expression::String(property_name.to_string());
        self.runtime_object_property_shadow_binding_by_property(owner_name, &property);
        self.runtime_object_property_shadow_deleted_binding_by_property(owner_name, &property);
    }

    pub(in crate::backend::direct_wasm) fn seed_runtime_shadow_cursor_owner_from_source(
        &mut self,
        target_owner: &str,
        source_owner: &str,
        source_expression: Option<&Expression>,
        properties: &[Expression],
    ) -> DirectResult<()> {
        if target_owner == source_owner || properties.is_empty() {
            return Ok(());
        }

        let mut seen_properties = HashSet::new();
        for property in properties {
            let property = self
                .resolve_property_key_expression(property)
                .unwrap_or_else(|| self.materialize_static_expression(property));
            let property = self.canonical_runtime_shadow_property_expression(&property);
            let shadow_key = Self::runtime_object_property_shadow_key(&property);
            if !seen_properties.insert(shadow_key.clone()) {
                continue;
            }

            self.runtime_object_property_shadow_binding_by_property(source_owner, &property);
            self.runtime_object_property_shadow_deleted_binding_by_property(
                source_owner,
                &property,
            );

            let shadow_value = self
                .runtime_object_property_shadow_static_value_for_owner(source_owner, &property)
                .or_else(|| {
                    let source_expression = source_expression?;
                    let member_expression = Expression::Member {
                        object: Box::new(source_expression.clone()),
                        property: Box::new(property.clone()),
                    };
                    let materialized = self.materialize_static_expression(&member_expression);
                    (!static_expression_matches(&materialized, &member_expression))
                        .then_some(materialized)
                });
            let Some(shadow_value) = shadow_value else {
                continue;
            };
            if Self::expression_is_runtime_object_property_shadow_identifier(&shadow_value)
                || !self.runtime_shadow_fallback_references_readable_bindings(&shadow_value)
            {
                continue;
            }

            let source_shadow_name = format!(
                "__ayy_object_property__{source_owner}__{}",
                Self::runtime_object_property_shadow_key(&property)
            );
            let materialized_value =
                self.reference_preserving_static_value_expression(&shadow_value);
            self.update_static_global_assignment_metadata(&source_shadow_name, &materialized_value);
        }

        self.clear_runtime_object_property_shadow_prefix(target_owner);
        self.clear_runtime_object_property_shadow_static_metadata_prefix(target_owner);
        self.emit_runtime_object_property_shadow_copy_to_exact_target(source_owner, target_owner)
    }

    fn runtime_object_property_name_from_shadow_suffix(suffix: &str) -> Option<String> {
        let hex = suffix.strip_prefix("str__")?;
        if hex.len() % 2 != 0 {
            return None;
        }
        let bytes = (0..hex.len())
            .step_by(2)
            .map(|index| u8::from_str_radix(&hex[index..index + 2], 16).ok())
            .collect::<Option<Vec<_>>>()?;
        String::from_utf8(bytes).ok()
    }

    pub(in crate::backend::direct_wasm) fn object_runtime_shadow_entries_from_binding(
        &self,
        object_binding: &ObjectValueBinding,
    ) -> Vec<(Expression, Expression)> {
        let mut entries = ordered_object_property_names(object_binding)
            .into_iter()
            .filter_map(|property_name| {
                let property = Expression::String(property_name.clone());
                let descriptor = object_binding_lookup_descriptor(object_binding, &property);
                let value = if descriptor.is_some_and(Self::property_descriptor_is_accessor) {
                    Some(Expression::Undefined)
                } else {
                    descriptor
                        .and_then(|descriptor| descriptor.value.clone())
                        .or_else(|| object_binding_lookup_value(object_binding, &property).cloned())
                };
                value.map(|value| (Expression::String(property_name), value))
            })
            .collect::<Vec<_>>();
        entries.extend(
            object_binding
                .symbol_properties
                .iter()
                .map(|(property, value)| {
                    (
                        self.canonical_runtime_shadow_property_expression(property),
                        value.clone(),
                    )
                }),
        );
        entries
    }

    fn property_descriptor_is_accessor(descriptor: &PropertyDescriptorBinding) -> bool {
        descriptor.has_get
            || descriptor.has_set
            || descriptor.getter.is_some()
            || descriptor.setter.is_some()
    }

    pub(in crate::backend::direct_wasm) fn expression_is_runtime_object_property_shadow_identifier(
        expression: &Expression,
    ) -> bool {
        matches!(
            expression,
            Expression::Identifier(name) if name.starts_with("__ayy_object_property__")
        )
    }

    fn runtime_shadow_class_entry_should_defer(
        target_owner: &str,
        fallback_value: &Expression,
    ) -> bool {
        if !Self::runtime_shadow_owner_is_class_object(target_owner) {
            return false;
        }

        let mut referenced_names = HashSet::new();
        collect_referenced_binding_names_from_expression(fallback_value, &mut referenced_names);
        referenced_names.contains(target_owner)
    }

    fn runtime_shadow_property_is_private(property: &Expression) -> bool {
        matches!(
            property,
            Expression::String(property_name)
                if property_name.starts_with("__ayy$private$")
                    || property_name.starts_with("__ayy$private_brand$")
        )
    }

    fn runtime_shadow_owner_is_class_object(owner_name: &str) -> bool {
        owner_name.starts_with("__ayy_class_expr_") || owner_name.starts_with("__ayy_class_ctor_")
    }

    fn runtime_shadow_owner_resolves_to_proxy(&self, owner_name: &str) -> bool {
        let expression = if owner_name == "this" {
            Expression::This
        } else {
            Expression::Identifier(owner_name.to_string())
        };
        self.resolve_proxy_binding_from_expression(&expression)
            .is_some()
    }

    fn runtime_shadow_owner_has_proxy_target_only_private_metadata(
        &self,
        owner_name: &str,
    ) -> bool {
        self.runtime_shadow_owner_resolves_to_proxy(owner_name)
    }

    fn filter_proxy_private_runtime_shadow_entries(
        &self,
        owner_name: &str,
        entries: &mut Vec<(Expression, Expression)>,
    ) {
        if self.runtime_shadow_owner_has_proxy_target_only_private_metadata(owner_name) {
            entries.retain(|(property, _)| !Self::runtime_shadow_property_is_private(property));
        }
    }

    fn filter_proxy_private_object_binding_entries(
        &self,
        owner_name: &str,
        object_binding: &mut ObjectValueBinding,
    ) {
        if !self.runtime_shadow_owner_has_proxy_target_only_private_metadata(owner_name) {
            return;
        }
        object_binding
            .string_properties
            .retain(|(property_name, _)| {
                !property_name.starts_with("__ayy$private$")
                    && !property_name.starts_with("__ayy$private_brand$")
            });
        object_binding
            .non_enumerable_string_properties
            .retain(|property_name| {
                !property_name.starts_with("__ayy$private$")
                    && !property_name.starts_with("__ayy$private_brand$")
            });
    }

    fn private_runtime_shadow_entries_for_owner(
        &self,
        source_owner: &str,
    ) -> Vec<(Expression, Expression)> {
        if self.should_suppress_private_runtime_shadow_fallbacks(source_owner) {
            return Vec::new();
        }
        if self.runtime_shadow_owner_has_proxy_target_only_private_metadata(source_owner) {
            return Vec::new();
        }
        let object_binding = if source_owner == "this" {
            self.resolve_home_object_this_binding()
                .or_else(|| self.resolve_object_binding_from_expression(&Expression::This))
        } else {
            self.resolve_object_binding_from_expression(&Expression::Identifier(
                source_owner.to_string(),
            ))
        };
        let Some(object_binding) = object_binding else {
            return Vec::new();
        };

        self.object_runtime_shadow_entries_from_binding(&object_binding)
            .into_iter()
            .filter(|(property, _)| {
                matches!(property, Expression::String(property_name) if property_name.starts_with("__ayy$private$"))
            })
            .collect()
    }

    fn class_init_private_runtime_shadow_entries_for_owner(
        &self,
        source_owner: &str,
    ) -> Vec<(Expression, Expression)> {
        if !source_owner.starts_with("__ayy_class_expr_")
            && !source_owner.starts_with("__ayy_class_ctor_")
        {
            return Vec::new();
        }

        // A class constructor binding name embeds the class binding it was
        // lowered for (`__ayy_class_ctor_<id>__name_<class binding>`); class
        // init bodies assign static private members through the class binding
        // identifier, so match either spelling of the owner.
        let class_binding_alias = source_owner
            .starts_with("__ayy_class_ctor_")
            .then(|| source_owner.rsplit_once("__name_").map(|(_, name)| name))
            .flatten()
            .filter(|name| name.starts_with("__ayy_class_expr_") || name.starts_with("__ayy_"));

        let mut entries = Vec::new();
        let mut seen = HashSet::new();
        for function in &self
            .backend
            .function_registry
            .catalog
            .registered_function_declarations
        {
            let returned_owner = function
                .name
                .starts_with("__ayy_class_init_")
                .then(|| {
                    function.body.iter().find_map(|statement| match statement {
                        Statement::Return(Expression::Identifier(name))
                            if name == source_owner
                                || class_binding_alias.is_some_and(|alias| alias == name) =>
                        {
                            Some(name.as_str())
                        }
                        _ => None,
                    })
                })
                .flatten();
            let Some(returned_owner) = returned_owner else {
                continue;
            };
            Self::collect_class_init_private_runtime_shadow_entries(
                returned_owner,
                &function.body,
                &mut seen,
                &mut entries,
            );
        }

        entries
    }

    fn collect_class_init_private_runtime_shadow_entries(
        source_owner: &str,
        statements: &[Statement],
        seen: &mut HashSet<String>,
        entries: &mut Vec<(Expression, Expression)>,
    ) {
        for statement in statements {
            match statement {
                Statement::Expression(Expression::Call { callee, arguments }) => {
                    Self::collect_class_init_private_define_property_entry(
                        source_owner,
                        callee,
                        arguments,
                        seen,
                        entries,
                    );
                }
                Statement::Expression(Expression::AssignMember {
                    object,
                    property,
                    value,
                }) => {
                    Self::collect_class_init_private_assignment_entry(
                        source_owner,
                        object,
                        property,
                        value,
                        seen,
                        entries,
                    );
                }
                Statement::AssignMember {
                    object,
                    property,
                    value,
                } => {
                    Self::collect_class_init_private_assignment_entry(
                        source_owner,
                        object,
                        property,
                        value,
                        seen,
                        entries,
                    );
                }
                Statement::Labeled { body, .. }
                | Statement::Block { body }
                | Statement::Declaration { body } => {
                    Self::collect_class_init_private_runtime_shadow_entries(
                        source_owner,
                        body,
                        seen,
                        entries,
                    );
                }
                Statement::If {
                    then_branch,
                    else_branch,
                    ..
                } => {
                    Self::collect_class_init_private_runtime_shadow_entries(
                        source_owner,
                        then_branch,
                        seen,
                        entries,
                    );
                    Self::collect_class_init_private_runtime_shadow_entries(
                        source_owner,
                        else_branch,
                        seen,
                        entries,
                    );
                }
                _ => {}
            }
        }
    }

    fn collect_class_init_private_assignment_entry(
        source_owner: &str,
        object: &Expression,
        property: &Expression,
        value: &Expression,
        seen: &mut HashSet<String>,
        entries: &mut Vec<(Expression, Expression)>,
    ) {
        if !matches!(object, Expression::Identifier(target) if target == source_owner) {
            return;
        }
        let Expression::String(property_name) = property else {
            return;
        };
        if !property_name.starts_with("__ayy$private$") {
            return;
        }
        if seen.insert(property_name.clone()) {
            entries.push((property.clone(), value.clone()));
        }
        if let Some(marker_property) = private_brand_marker_property_expression(property)
            && let Expression::String(marker_name) = &marker_property
            && seen.insert(marker_name.clone())
        {
            entries.push((marker_property, Expression::Bool(true)));
        }
    }

    fn collect_class_init_private_define_property_entry(
        source_owner: &str,
        callee: &Expression,
        arguments: &[CallArgument],
        seen: &mut HashSet<String>,
        entries: &mut Vec<(Expression, Expression)>,
    ) {
        let Expression::Member { object, property } = callee else {
            return;
        };
        if !matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
            || !matches!(property.as_ref(), Expression::String(name) if name == "defineProperty")
        {
            return;
        }
        let [
            CallArgument::Expression(Expression::Identifier(target)),
            CallArgument::Expression(property),
            CallArgument::Expression(descriptor_expression),
            ..,
        ] = arguments
        else {
            return;
        };
        if target != source_owner {
            return;
        }
        let Expression::String(property_name) = property else {
            return;
        };
        if !property_name.starts_with("__ayy$private$") {
            return;
        }
        let Some(descriptor) = resolve_property_descriptor_definition(descriptor_expression) else {
            return;
        };
        let fallback_value = descriptor
            .value
            .or(descriptor.getter)
            .or(descriptor.setter)
            .unwrap_or(Expression::Undefined);
        let property = Expression::String(property_name.clone());
        if seen.insert(property_name.clone()) {
            entries.push((property.clone(), fallback_value));
        }
        if let Some(marker_property) = private_brand_marker_property_expression(&property)
            && let Expression::String(marker_name) = &marker_property
            && seen.insert(marker_name.clone())
        {
            entries.push((marker_property, Expression::Bool(true)));
        }
    }

    fn private_runtime_shadow_marker_fallback(
        &self,
        source_owner: &str,
        property: &Expression,
        fallback_value: Expression,
    ) -> Expression {
        if !matches!(property, Expression::String(property_name) if property_name.starts_with("__ayy$private$"))
            || !matches!(fallback_value, Expression::Undefined)
        {
            return fallback_value;
        }

        let source_expression = if source_owner == "this" {
            Expression::This
        } else {
            Expression::Identifier(source_owner.to_string())
        };
        self.resolve_member_getter_binding(&source_expression, property)
            .or_else(|| self.resolve_member_setter_binding(&source_expression, property))
            .or_else(|| self.resolve_member_function_binding(&source_expression, property))
            .map(|binding| match binding {
                LocalFunctionBinding::User(function_name)
                | LocalFunctionBinding::Builtin(function_name) => {
                    Expression::Identifier(function_name)
                }
            })
            .unwrap_or(fallback_value)
    }

    fn private_brand_marker_capture_slot_for_owner(&self, owner: &str) -> Option<String> {
        let local_capture_slots = self
            .state
            .speculation
            .static_semantics
            .objects
            .member_function_capture_slots
            .iter()
            .map(|(key, slots)| (key.clone(), slots.clone()));
        let global_capture_slots = self.backend.global_member_function_capture_slot_entries();
        let mut slots = BTreeSet::new();
        for (key, capture_slots) in local_capture_slots.chain(global_capture_slots) {
            if !matches!(
                &key.target,
                MemberFunctionBindingTarget::Identifier(target) if target == owner
            ) {
                continue;
            }
            for (capture_name, slot_name) in capture_slots {
                if capture_name.starts_with("__ayy_class_brand_") {
                    slots.insert(slot_name);
                }
            }
        }
        if slots.len() == 1 {
            slots.into_iter().next()
        } else {
            None
        }
    }

    fn private_brand_marker_copy_fallback_for_target(
        &self,
        target_owner: &str,
        property: &Expression,
        fallback_value: &Expression,
    ) -> Option<Expression> {
        if !matches!(fallback_value, Expression::Bool(true)) {
            return None;
        }
        let Expression::String(property_name) = property else {
            return None;
        };
        if !property_name.starts_with("__ayy$private_brand$") {
            return None;
        }
        self.private_brand_marker_capture_slot_for_owner(target_owner)
            .map(Expression::Identifier)
    }

    fn append_static_private_marker_shadow_entries(
        source_owner: &str,
        entries: &mut Vec<(Expression, Option<Expression>)>,
        known_private_properties: &mut HashSet<String>,
    ) {
        if source_owner == "this" {
            return;
        }
        let mut marker_entries = Vec::new();
        for (property, _) in entries.iter() {
            let Expression::String(property_name) = property else {
                continue;
            };
            if !property_name.starts_with("__ayy$private$")
                || (!property_name.contains("__ayy_class_expr_")
                    && !property_name.contains("__ayy_class_ctor_"))
            {
                continue;
            }
            let Some(marker_property) = private_brand_marker_property_expression(property) else {
                continue;
            };
            let Expression::String(marker_name) = &marker_property else {
                continue;
            };
            if known_private_properties.insert(marker_name.clone()) {
                marker_entries.push((marker_property, Some(Expression::Bool(true))));
            }
        }
        entries.extend(marker_entries);
    }

    fn class_init_defines_static_private_marker(
        &self,
        source_owner: &str,
        marker_property: &Expression,
    ) -> bool {
        self.class_init_private_runtime_shadow_entries_for_owner(source_owner)
            .into_iter()
            .any(|(property, value)| {
                static_expression_matches(&property, marker_property)
                    && matches!(value, Expression::Bool(true))
            })
    }

    pub(in crate::backend::direct_wasm) fn runtime_object_property_shadow_copy_entries(
        &self,
        source_owner: &str,
    ) -> Vec<(Expression, Option<Expression>)> {
        let suppress_private_fallbacks =
            self.should_suppress_private_runtime_shadow_fallbacks(source_owner);
        let mut entries = self
            .object_runtime_shadow_properties(source_owner)
            .into_iter()
            .filter(|(property, _)| {
                !suppress_private_fallbacks || !Self::runtime_shadow_property_is_private(property)
            })
            .map(|(property, fallback_value)| {
                let fallback_value = self.private_runtime_shadow_marker_fallback(
                    source_owner,
                    &property,
                    fallback_value,
                );
                (property, Some(fallback_value))
            })
            .collect::<Vec<_>>();

        let mut known_property_keys = entries
            .iter()
            .map(|(property, _)| Self::runtime_object_property_shadow_key(property))
            .collect::<HashSet<_>>();
        let property_prefix = format!("__ayy_object_property__{source_owner}__");
        let predeclared_shadow_names = self
            .active_runtime_object_shadow_names_with_prefix(&property_prefix)
            .into_iter()
            .map(|(_, name)| name)
            .collect::<Vec<_>>();
        for name in predeclared_shadow_names {
            let Some(property_name) = Self::runtime_object_property_name_from_shadow_suffix(
                &name[property_prefix.len()..],
            ) else {
                continue;
            };
            let property = Expression::String(property_name);
            if known_property_keys.insert(Self::runtime_object_property_shadow_key(&property)) {
                entries.push((property, None));
            }
        }
        if let Some(source_binding) = self.resolve_runtime_shadow_object_binding(source_owner) {
            let mut shadow_entries =
                self.object_runtime_shadow_entries_from_binding(&source_binding);
            self.filter_proxy_private_runtime_shadow_entries(source_owner, &mut shadow_entries);
            for (property, fallback_value) in shadow_entries {
                if suppress_private_fallbacks && Self::runtime_shadow_property_is_private(&property)
                {
                    continue;
                }
                let property_key = Self::runtime_object_property_shadow_key(&property);
                let fallback_value = self.private_runtime_shadow_marker_fallback(
                    source_owner,
                    &property,
                    fallback_value,
                );
                if !known_property_keys.insert(property_key.clone()) {
                    if let Some((_, existing_fallback)) =
                        entries.iter_mut().find(|(existing_property, _)| {
                            Self::runtime_object_property_shadow_key(existing_property)
                                == property_key
                        })
                    {
                        *existing_fallback = Some(fallback_value);
                    }
                    continue;
                }
                entries.push((property, Some(fallback_value)));
            }
        }

        let mut known_private_properties = entries
            .iter()
            .filter_map(|(property, _)| match property {
                Expression::String(property_name) => Some(property_name.clone()),
                _ => None,
            })
            .collect::<HashSet<_>>();
        for (property, fallback_value) in
            self.private_runtime_shadow_entries_for_owner(source_owner)
        {
            let Expression::String(property_name) = &property else {
                continue;
            };
            if known_private_properties.insert(property_name.clone()) {
                let fallback_value = self.private_runtime_shadow_marker_fallback(
                    source_owner,
                    &property,
                    fallback_value,
                );
                entries.push((property, Some(fallback_value)));
            }
        }
        for (property, fallback_value) in
            self.class_init_private_runtime_shadow_entries_for_owner(source_owner)
        {
            let Expression::String(property_name) = &property else {
                continue;
            };
            if known_private_properties.insert(property_name.clone()) {
                entries.push((property, Some(fallback_value)));
            }
        }
        Self::append_static_private_marker_shadow_entries(
            source_owner,
            &mut entries,
            &mut known_private_properties,
        );
        self.seed_static_private_brand_marker_fallbacks(source_owner, &mut entries);
        for (property, fallback_value) in &mut entries {
            let Some(shadow_value) =
                self.runtime_object_property_shadow_static_value_for_owner(source_owner, property)
            else {
                continue;
            };
            if Self::expression_is_runtime_object_property_shadow_identifier(&shadow_value)
                || !self.runtime_shadow_fallback_references_readable_bindings(&shadow_value)
            {
                continue;
            }
            *fallback_value = Some(shadow_value);
        }

        entries
    }

    /// When copying shadows from a class binding (the class object itself),
    /// give `__ayy$private_brand$` marker entries without a runtime source a
    /// fallback that evaluates to the class's private brand. Static private
    /// methods and accessors are defined on the class object via property
    /// descriptors, so nothing ever stamps their brand marker shadow at class
    /// definition time the way instance constructors do.
    fn seed_static_private_brand_marker_fallbacks(
        &self,
        source_owner: &str,
        entries: &mut Vec<(Expression, Option<Expression>)>,
    ) {
        if source_owner == "this" {
            return;
        }
        let source_expression = Expression::Identifier(source_owner.to_string());
        let Some(LocalFunctionBinding::User(constructor_name)) =
            self.resolve_function_binding_from_expression(&source_expression)
        else {
            return;
        };
        let Some(brand_binding) = self
            .user_function(&constructor_name)
            .and_then(|function| function.private_brand_binding.clone())
        else {
            return;
        };
        let source_has_private_member = |compiler: &Self, private_property: &Expression| {
            compiler
                .resolve_member_getter_binding(&source_expression, private_property)
                .is_some()
                || compiler
                    .resolve_member_setter_binding(&source_expression, private_property)
                    .is_some()
                || compiler
                    .resolve_member_function_binding(&source_expression, private_property)
                    .is_some()
        };
        let mut known_marker_names = HashSet::new();
        let mut missing_marker_entries = Vec::new();
        for (property, fallback_value) in entries.iter_mut() {
            let Expression::String(property_name) = property else {
                continue;
            };
            if let Some(private_property_name) = property_name.strip_prefix("__ayy$private_brand$")
            {
                known_marker_names.insert(property_name.clone());
                if fallback_value.is_some() {
                    continue;
                }
                let private_property = Expression::String(private_property_name.to_string());
                if !source_has_private_member(self, &private_property) {
                    continue;
                }
                *fallback_value = Some(Expression::Identifier(brand_binding.clone()));
            } else if property_name.starts_with("__ayy$private$")
                && source_has_private_member(self, property)
                && let Some(marker_property) = private_brand_marker_property_expression(property)
            {
                missing_marker_entries.push(marker_property);
            }
        }
        for marker_property in missing_marker_entries {
            let Expression::String(marker_name) = &marker_property else {
                continue;
            };
            if known_marker_names.insert(marker_name.clone()) {
                entries.push((
                    marker_property,
                    Some(Expression::Identifier(brand_binding.clone())),
                ));
            }
        }
    }

    fn append_target_private_runtime_shadow_copy_entries(
        &self,
        source_owner: &str,
        target_owner: &str,
        entries: &mut Vec<(Expression, Option<Expression>)>,
    ) {
        let mut known_suffixes = entries
            .iter()
            .map(|(property, _)| Self::runtime_object_property_shadow_key(property))
            .collect::<HashSet<_>>();
        let mut private_properties = BTreeSet::new();
        let target_prefix = format!("__ayy_object_property__{target_owner}__");
        for name in self
            .backend
            .global_semantics
            .global_names()
            .implicit_bindings
            .keys()
        {
            if let Some((suffix, property_name)) =
                name.strip_prefix(&target_prefix).and_then(|suffix| {
                    let property_name =
                        Self::runtime_object_property_name_from_shadow_suffix(suffix)?;
                    (property_name.starts_with("__ayy$private$")
                        || property_name.starts_with("__ayy$private_brand$"))
                    .then_some((suffix.to_string(), property_name))
                })
            {
                private_properties.insert((suffix, property_name));
            }

            if self.should_suppress_private_runtime_shadow_fallbacks(source_owner)
                && name.starts_with("__ayy_object_property__")
            {
                for (index, _) in name.match_indices("__str__") {
                    let suffix = &name[index + 2..];
                    let Some(property_name) =
                        Self::runtime_object_property_name_from_shadow_suffix(suffix)
                    else {
                        continue;
                    };
                    if property_name.starts_with("__ayy$private$")
                        || property_name.starts_with("__ayy$private_brand$")
                    {
                        private_properties.insert((suffix.to_string(), property_name));
                    }
                }
            }
        }

        for (suffix, property_name) in private_properties {
            if known_suffixes.insert(suffix) {
                entries.push((Expression::String(property_name), None));
            }
        }
        self.seed_static_private_brand_marker_fallbacks(source_owner, entries);
    }

    fn should_suppress_private_runtime_shadow_fallbacks(&self, source_owner: &str) -> bool {
        source_owner == "this"
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_symbol_property_shadow_entry(
        &self,
        object_binding: &ObjectValueBinding,
        property: &Expression,
    ) -> Option<(Expression, Expression)> {
        let canonical_property = self.canonical_object_property_expression(property);
        let requested_symbol = self
            .resolve_symbol_identity_expression(&canonical_property)
            .or_else(|| self.resolve_symbol_identity_expression(property));

        object_binding
            .symbol_properties
            .iter()
            .find_map(|(existing_key, fallback_value)| {
                let canonical_existing = self.canonical_object_property_expression(existing_key);
                if static_expression_matches(&canonical_existing, &canonical_property)
                    || static_expression_matches(existing_key, property)
                {
                    return Some((existing_key.clone(), fallback_value.clone()));
                }

                let requested_symbol = requested_symbol.as_ref()?;
                let existing_symbol = self
                    .resolve_symbol_identity_expression(&canonical_existing)
                    .or_else(|| self.resolve_symbol_identity_expression(existing_key))?;
                static_expression_matches(&existing_symbol, requested_symbol)
                    .then_some((existing_key.clone(), fallback_value.clone()))
            })
    }

    fn expression_references_any_parameter(
        expression: &Expression,
        parameter_names: &HashSet<String>,
    ) -> bool {
        match expression {
            Expression::Identifier(name) => parameter_names.contains(name),
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(value) | ArrayElement::Spread(value) => {
                    Self::expression_references_any_parameter(value, parameter_names)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    Self::expression_references_any_parameter(key, parameter_names)
                        || Self::expression_references_any_parameter(value, parameter_names)
                }
                ObjectEntry::Getter { key, getter } => {
                    Self::expression_references_any_parameter(key, parameter_names)
                        || Self::expression_references_any_parameter(getter, parameter_names)
                }
                ObjectEntry::Setter { key, setter } => {
                    Self::expression_references_any_parameter(key, parameter_names)
                        || Self::expression_references_any_parameter(setter, parameter_names)
                }
                ObjectEntry::Spread(value) => {
                    Self::expression_references_any_parameter(value, parameter_names)
                }
            }),
            Expression::Member { object, property } => {
                Self::expression_references_any_parameter(object, parameter_names)
                    || Self::expression_references_any_parameter(property, parameter_names)
            }
            Expression::SuperMember { property } => {
                Self::expression_references_any_parameter(property, parameter_names)
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => Self::expression_references_any_parameter(value, parameter_names),
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                Self::expression_references_any_parameter(object, parameter_names)
                    || Self::expression_references_any_parameter(property, parameter_names)
                    || Self::expression_references_any_parameter(value, parameter_names)
            }
            Expression::AssignSuperMember { property, value } => {
                Self::expression_references_any_parameter(property, parameter_names)
                    || Self::expression_references_any_parameter(value, parameter_names)
            }
            Expression::Binary { left, right, .. } => {
                Self::expression_references_any_parameter(left, parameter_names)
                    || Self::expression_references_any_parameter(right, parameter_names)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::expression_references_any_parameter(condition, parameter_names)
                    || Self::expression_references_any_parameter(then_expression, parameter_names)
                    || Self::expression_references_any_parameter(else_expression, parameter_names)
            }
            Expression::Sequence(expressions) => expressions.iter().any(|expression| {
                Self::expression_references_any_parameter(expression, parameter_names)
            }),
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                Self::expression_references_any_parameter(callee, parameter_names)
                    || arguments.iter().any(|argument| {
                        Self::expression_references_any_parameter(
                            argument.expression(),
                            parameter_names,
                        )
                    })
            }
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::NewTarget
            | Expression::This
            | Expression::Sent
            | Expression::Update { .. } => false,
        }
    }

    fn expression_writes_parameter_member(
        expression: &Expression,
        parameter_names: &HashSet<String>,
    ) -> bool {
        match expression {
            Expression::AssignMember { object, value, .. } => {
                Self::expression_references_any_parameter(object, parameter_names)
                    || Self::expression_writes_parameter_member(value, parameter_names)
            }
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(value) | ArrayElement::Spread(value) => {
                    Self::expression_writes_parameter_member(value, parameter_names)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    Self::expression_writes_parameter_member(key, parameter_names)
                        || Self::expression_writes_parameter_member(value, parameter_names)
                }
                ObjectEntry::Getter { key, getter } => {
                    Self::expression_writes_parameter_member(key, parameter_names)
                        || Self::expression_writes_parameter_member(getter, parameter_names)
                }
                ObjectEntry::Setter { key, setter } => {
                    Self::expression_writes_parameter_member(key, parameter_names)
                        || Self::expression_writes_parameter_member(setter, parameter_names)
                }
                ObjectEntry::Spread(value) => {
                    Self::expression_writes_parameter_member(value, parameter_names)
                }
            }),
            Expression::Member { object, property } => {
                Self::expression_writes_parameter_member(object, parameter_names)
                    || Self::expression_writes_parameter_member(property, parameter_names)
            }
            Expression::SuperMember { property } => {
                Self::expression_writes_parameter_member(property, parameter_names)
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => Self::expression_writes_parameter_member(value, parameter_names),
            Expression::AssignSuperMember { property, value } => {
                Self::expression_writes_parameter_member(property, parameter_names)
                    || Self::expression_writes_parameter_member(value, parameter_names)
            }
            Expression::Binary { left, right, .. } => {
                Self::expression_writes_parameter_member(left, parameter_names)
                    || Self::expression_writes_parameter_member(right, parameter_names)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::expression_writes_parameter_member(condition, parameter_names)
                    || Self::expression_writes_parameter_member(then_expression, parameter_names)
                    || Self::expression_writes_parameter_member(else_expression, parameter_names)
            }
            Expression::Sequence(expressions) => expressions.iter().any(|expression| {
                Self::expression_writes_parameter_member(expression, parameter_names)
            }),
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                Self::expression_writes_parameter_member(callee, parameter_names)
                    || arguments.iter().any(|argument| {
                        Self::expression_writes_parameter_member(
                            argument.expression(),
                            parameter_names,
                        )
                    })
            }
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::Identifier(_)
            | Expression::NewTarget
            | Expression::This
            | Expression::Sent
            | Expression::Update { .. } => false,
        }
    }

    fn statement_writes_parameter_member(
        statement: &Statement,
        parameter_names: &HashSet<String>,
    ) -> bool {
        match statement {
            Statement::AssignMember { object, value, .. } => {
                Self::expression_references_any_parameter(object, parameter_names)
                    || Self::expression_writes_parameter_member(value, parameter_names)
            }
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => body.iter().any(|statement| {
                Self::statement_writes_parameter_member(statement, parameter_names)
            }),
            Statement::Var { value, .. }
            | Statement::Let { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Expression(value)
            | Statement::Throw(value)
            | Statement::Return(value)
            | Statement::Yield { value }
            | Statement::YieldDelegate { value } => {
                Self::expression_writes_parameter_member(value, parameter_names)
            }
            Statement::Print { values } => values
                .iter()
                .any(|value| Self::expression_writes_parameter_member(value, parameter_names)),
            Statement::With { .. } => true,
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                Self::expression_writes_parameter_member(condition, parameter_names)
                    || then_branch.iter().any(|statement| {
                        Self::statement_writes_parameter_member(statement, parameter_names)
                    })
                    || else_branch.iter().any(|statement| {
                        Self::statement_writes_parameter_member(statement, parameter_names)
                    })
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => body
                .iter()
                .chain(catch_setup)
                .chain(catch_body)
                .any(|statement| {
                    Self::statement_writes_parameter_member(statement, parameter_names)
                }),
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                Self::expression_writes_parameter_member(discriminant, parameter_names)
                    || cases.iter().any(|case| {
                        case.body.iter().any(|statement| {
                            Self::statement_writes_parameter_member(statement, parameter_names)
                        })
                    })
            }
            Statement::For {
                init,
                condition,
                update,
                break_hook,
                body,
                ..
            } => {
                init.iter().any(|statement| {
                    Self::statement_writes_parameter_member(statement, parameter_names)
                }) || condition.as_ref().is_some_and(|value| {
                    Self::expression_writes_parameter_member(value, parameter_names)
                }) || update.as_ref().is_some_and(|value| {
                    Self::expression_writes_parameter_member(value, parameter_names)
                }) || break_hook.as_ref().is_some_and(|value| {
                    Self::expression_writes_parameter_member(value, parameter_names)
                }) || body.iter().any(|statement| {
                    Self::statement_writes_parameter_member(statement, parameter_names)
                })
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
                Self::expression_writes_parameter_member(condition, parameter_names)
                    || break_hook.as_ref().is_some_and(|value| {
                        Self::expression_writes_parameter_member(value, parameter_names)
                    })
                    || body.iter().any(|statement| {
                        Self::statement_writes_parameter_member(statement, parameter_names)
                    })
            }
            Statement::Break { .. } | Statement::Continue { .. } => false,
        }
    }

    fn user_function_may_need_parameter_object_shadow_setup(
        &self,
        user_function: &UserFunction,
    ) -> bool {
        if self.user_function_mentions_direct_eval(user_function)
            || self.user_function_mentions_private_member_access(user_function)
        {
            return true;
        }
        let Some(function) = self.resolve_registered_function_declaration(&user_function.name)
        else {
            return true;
        };
        let parameter_names = user_function.params.iter().cloned().collect::<HashSet<_>>();
        function
            .body
            .iter()
            .any(|statement| Self::statement_writes_parameter_member(statement, &parameter_names))
    }

    pub(in crate::backend::direct_wasm) fn emit_user_function_parameter_object_shadow_setup(
        &mut self,
        user_function: &UserFunction,
        argument_expressions: &[Expression],
    ) -> DirectResult<Vec<(String, String, Option<ObjectValueBinding>)>> {
        if !self.user_function_may_need_parameter_object_shadow_setup(user_function) {
            return Ok(Vec::new());
        }
        let parameter_bindings = self
            .backend
            .function_registry
            .parameter_bindings_for(&user_function.name);
        let mut writebacks = Vec::new();

        for (index, param_name) in user_function.params.iter().enumerate() {
            let Some(argument_expression) = argument_expressions.get(index) else {
                continue;
            };
            let argument_requires_current_object_binding = matches!(argument_expression, Expression::Object(entries) if entries.iter().any(|entry| matches!(entry, ObjectEntry::Spread(_))));
            let argument_reads_descriptor_member =
                self.expression_reads_local_descriptor_binding_member(argument_expression);
            let argument_contains_await =
                Self::expression_contains_await_for_user_call_runtime(argument_expression);
            let parameter_object_binding = if argument_reads_descriptor_member
                || argument_requires_current_object_binding
                || argument_contains_await
            {
                None
            } else {
                parameter_bindings
                    .object_bindings
                    .get(param_name)
                    .and_then(|binding| binding.as_ref())
            };

            let source_owner = match argument_expression {
                Expression::Identifier(name) => {
                    self.runtime_object_property_shadow_owner_name_for_identifier(name)
                }
                _ => None,
            };
            let argument_object_binding = if argument_reads_descriptor_member
                || argument_contains_await
            {
                None
            } else {
                self.resolve_object_binding_from_expression(argument_expression)
                    .map(|binding| {
                        let binding = self.object_binding_with_constructed_constructor_shadow(
                            binding,
                            argument_expression,
                        );
                        self.object_binding_with_function_argument_metadata(
                            binding,
                            argument_expression,
                        )
                    })
                    .or_else(|| self.function_argument_metadata_object_binding(argument_expression))
            };
            if crate::ayy_env_flag!("AYY_TRACE_PRIVATE_MEMBER_LOOKUP") {
                eprintln!(
                    "private_param_shadow_setup fn={} param={} arg={argument_expression:?} descriptor_arg={} param_binding={} arg_binding={} source_owner={source_owner:?}",
                    user_function.name,
                    param_name,
                    argument_reads_descriptor_member,
                    parameter_object_binding.is_some(),
                    argument_object_binding.is_some(),
                );
            }
            if parameter_object_binding.is_none()
                && argument_object_binding.is_none()
                && source_owner.is_none()
            {
                continue;
            }
            if source_owner.as_deref() == Some(param_name.as_str()) {
                continue;
            }
            self.clear_runtime_object_property_shadow_prefix(param_name);
            self.clear_runtime_object_property_shadow_static_metadata_prefix(param_name);
            self.state.clear_member_bindings_for_name(param_name, true);
            if self.binding_name_is_global(param_name) {
                self.backend
                    .clear_global_member_bindings_for_name(param_name);
            }
            if let Some(source_owner) = source_owner.as_ref() {
                let source_owner_has_bindings =
                    self.runtime_object_property_shadow_owner_has_bindings(source_owner);
                self.emit_runtime_object_property_shadow_copy(source_owner, param_name)?;
                if let Some(argument_object_binding) = argument_object_binding
                    .as_ref()
                    .or(parameter_object_binding)
                {
                    if let Some((resolved_param_name, _)) =
                        self.resolve_current_local_binding(param_name)
                    {
                        if resolved_param_name != *param_name {
                            self.state
                                .speculation
                                .static_semantics
                                .set_local_object_binding(
                                    &resolved_param_name,
                                    argument_object_binding.clone(),
                                );
                        }
                    }
                    self.state
                        .speculation
                        .static_semantics
                        .set_local_object_binding(param_name, argument_object_binding.clone());
                    if !source_owner_has_bindings {
                        let getter_this_expression = if source_owner == "this" {
                            Expression::This
                        } else {
                            Expression::Identifier(source_owner.clone())
                        };
                        self.emit_runtime_object_property_shadow_seed_from_binding_with_receiver(
                            param_name,
                            argument_object_binding,
                            &getter_this_expression,
                        )?;
                    }
                    self.sync_runtime_object_property_shadow_static_metadata_from_binding(
                        param_name,
                        argument_object_binding,
                    );
                }
                self.copy_member_bindings_for_alias(param_name, source_owner);
                writebacks.push((
                    param_name.clone(),
                    source_owner.clone(),
                    argument_object_binding.clone(),
                ));
                continue;
            }

            if let Some(argument_object_binding) = argument_object_binding
                .as_ref()
                .or(parameter_object_binding)
            {
                if let Some((resolved_param_name, _)) =
                    self.resolve_current_local_binding(param_name)
                {
                    if resolved_param_name != *param_name {
                        self.state
                            .speculation
                            .static_semantics
                            .set_local_object_binding(
                                &resolved_param_name,
                                argument_object_binding.clone(),
                            );
                    }
                }
                self.state
                    .speculation
                    .static_semantics
                    .set_local_object_binding(param_name, argument_object_binding.clone());
                self.emit_runtime_object_property_shadow_seed_from_binding_with_receiver(
                    param_name,
                    argument_object_binding,
                    argument_expression,
                )?;
                self.sync_runtime_object_property_shadow_static_metadata_from_binding(
                    param_name,
                    argument_object_binding,
                );
            }
        }

        for index in &user_function.extra_argument_indices {
            let Some(argument_expression) = argument_expressions.get(*index as usize) else {
                continue;
            };
            let owner_name = Self::user_function_arguments_slot_object_shadow_owner_name(
                &user_function.name,
                *index,
            );
            let argument_reads_descriptor_member =
                self.expression_reads_local_descriptor_binding_member(argument_expression);
            let argument_contains_await =
                Self::expression_contains_await_for_user_call_runtime(argument_expression);
            let argument_requires_current_object_binding = matches!(argument_expression, Expression::Object(entries) if entries.iter().any(|entry| matches!(entry, ObjectEntry::Spread(_))));
            let source_owner = match argument_expression {
                Expression::Identifier(name) => {
                    self.runtime_object_property_shadow_owner_name_for_identifier(name)
                }
                Expression::This => {
                    self.runtime_object_property_shadow_owner_name_for_identifier("this")
                }
                _ => None,
            };
            let argument_object_binding = if argument_reads_descriptor_member
                || argument_contains_await
            {
                None
            } else {
                self.resolve_object_binding_from_expression(argument_expression)
                    .map(|binding| {
                        let binding = self.object_binding_with_constructed_constructor_shadow(
                            binding,
                            argument_expression,
                        );
                        self.object_binding_with_function_argument_metadata(
                            binding,
                            argument_expression,
                        )
                    })
                    .or_else(|| {
                        (!argument_requires_current_object_binding)
                            .then(|| {
                                self.function_argument_metadata_object_binding(argument_expression)
                            })
                            .flatten()
                    })
            };

            if argument_object_binding.is_none() && source_owner.is_none() {
                continue;
            }
            if source_owner.as_deref() == Some(owner_name.as_str()) {
                continue;
            }
            self.clear_runtime_object_property_shadow_prefix(&owner_name);
            self.clear_runtime_object_property_shadow_static_metadata_prefix(&owner_name);
            self.state.clear_member_bindings_for_name(&owner_name, true);
            if self.binding_name_is_global(&owner_name) {
                self.backend
                    .clear_global_member_bindings_for_name(&owner_name);
            }
            for property_name in
                Self::direct_arguments_slot_assignment_properties(user_function, *index)
            {
                self.predeclare_runtime_shadow_property(&owner_name, &property_name);
            }
            if let Some(source_owner) = source_owner.as_ref() {
                let source_owner_has_bindings =
                    self.runtime_object_property_shadow_owner_has_bindings(source_owner);
                self.emit_runtime_object_property_shadow_copy(source_owner, &owner_name)?;
                if let Some(argument_object_binding) = argument_object_binding.as_ref() {
                    if !source_owner_has_bindings {
                        let getter_this_expression = if source_owner == "this" {
                            Expression::This
                        } else {
                            Expression::Identifier(source_owner.clone())
                        };
                        self.emit_runtime_object_property_shadow_seed_from_binding_with_receiver(
                            &owner_name,
                            argument_object_binding,
                            &getter_this_expression,
                        )?;
                    }
                    self.sync_runtime_object_property_shadow_static_metadata_from_binding(
                        &owner_name,
                        argument_object_binding,
                    );
                }
                self.copy_member_bindings_for_alias(&owner_name, source_owner);
                writebacks.push((
                    owner_name,
                    source_owner.clone(),
                    argument_object_binding.clone(),
                ));
                continue;
            }

            if let Some(argument_object_binding) = argument_object_binding.as_ref() {
                self.emit_runtime_object_property_shadow_seed_from_binding_with_receiver(
                    &owner_name,
                    argument_object_binding,
                    argument_expression,
                )?;
                self.sync_runtime_object_property_shadow_static_metadata_from_binding(
                    &owner_name,
                    argument_object_binding,
                );
            }
        }

        Ok(writebacks)
    }

    fn function_argument_metadata_object_binding(
        &self,
        argument_expression: &Expression,
    ) -> Option<ObjectValueBinding> {
        let function_binding = self.resolve_function_binding_from_expression(argument_expression);
        let mut object_binding = empty_object_value_binding();
        let (name_value, length_value) = match function_binding {
            Some(LocalFunctionBinding::User(function_name)) => (
                self.user_function(&function_name)
                    .and_then(|user_function| {
                        self.runtime_user_function_property_value(user_function, "name")
                    })
                    .or_else(|| {
                        self.runtime_registered_function_property_value(&function_name, "name")
                    }),
                self.user_function(&function_name)
                    .and_then(|user_function| {
                        self.runtime_user_function_property_value(user_function, "length")
                    })
                    .or_else(|| {
                        self.runtime_registered_function_property_value(&function_name, "length")
                    }),
            ),
            Some(LocalFunctionBinding::Builtin(function_name)) => (
                Some(Expression::String(
                    builtin_function_display_name(&function_name).to_string(),
                )),
                builtin_function_length(&function_name)
                    .map(|length| Expression::Number(length as f64)),
            ),
            None => {
                let name_member = Expression::Member {
                    object: Box::new(argument_expression.clone()),
                    property: Box::new(Expression::String("name".to_string())),
                };
                let length_member = Expression::Member {
                    object: Box::new(argument_expression.clone()),
                    property: Box::new(Expression::String("length".to_string())),
                };
                let hinted_user_function =
                    if let Expression::Identifier(argument_name) = argument_expression {
                        let source_name =
                            scoped_binding_source_name(argument_name).unwrap_or(argument_name);
                        let matches = self
                            .user_functions()
                            .into_iter()
                            .filter(|function| {
                                self.resolve_user_function_display_name(&function.name)
                                    .as_deref()
                                    == Some(source_name)
                            })
                            .collect::<Vec<_>>();
                        match matches.as_slice() {
                            [function] => Some(function.clone()),
                            _ => None,
                        }
                    } else {
                        None
                    };
                (
                    self.resolve_static_string_value(&name_member)
                        .map(Expression::String)
                        .or_else(|| {
                            hinted_user_function.as_ref().and_then(|function| {
                                self.runtime_user_function_property_value(function, "name")
                            })
                        }),
                    self.resolve_static_number_value(&length_member)
                        .map(Expression::Number)
                        .or_else(|| {
                            hinted_user_function.as_ref().and_then(|function| {
                                self.runtime_user_function_property_value(function, "length")
                            })
                        }),
                )
            }
        };
        if let Some(name_value) = name_value {
            object_binding_define_property_descriptor(
                &mut object_binding,
                Expression::String("name".to_string()),
                PropertyDescriptorBinding {
                    value: Some(name_value),
                    configurable: true,
                    enumerable: false,
                    writable: Some(false),
                    getter: None,
                    setter: None,
                    has_get: false,
                    has_set: false,
                },
            );
        }
        if let Some(length_value) = length_value {
            object_binding_define_property_descriptor(
                &mut object_binding,
                Expression::String("length".to_string()),
                PropertyDescriptorBinding {
                    value: Some(length_value),
                    configurable: true,
                    enumerable: false,
                    writable: Some(false),
                    getter: None,
                    setter: None,
                    has_get: false,
                    has_set: false,
                },
            );
        }
        (!object_binding.string_properties.is_empty()
            || !object_binding.property_descriptors.is_empty())
        .then_some(object_binding)
    }

    fn object_binding_with_function_argument_metadata(
        &self,
        mut object_binding: ObjectValueBinding,
        argument_expression: &Expression,
    ) -> ObjectValueBinding {
        let Some(function_metadata) =
            self.function_argument_metadata_object_binding(argument_expression)
        else {
            return object_binding;
        };
        for (property, descriptor) in function_metadata.property_descriptors {
            object_binding_define_property_descriptor(&mut object_binding, property, descriptor);
        }
        object_binding
    }

    pub(in crate::backend::direct_wasm) fn object_binding_with_constructed_constructor_shadow(
        &self,
        mut object_binding: ObjectValueBinding,
        argument_expression: &Expression,
    ) -> ObjectValueBinding {
        if object_binding
            .string_properties
            .iter()
            .any(|(property, _)| property == "constructor")
        {
            return object_binding;
        }
        let Some(constructor_binding) =
            self.constructed_object_constructor_binding_for_shadow_argument(argument_expression)
        else {
            return object_binding;
        };
        let constructor_expression = match constructor_binding {
            LocalFunctionBinding::User(function_name)
            | LocalFunctionBinding::Builtin(function_name) => Expression::Identifier(function_name),
        };
        object_binding
            .string_properties
            .push(("constructor".to_string(), constructor_expression));
        if !object_binding
            .non_enumerable_string_properties
            .iter()
            .any(|property| property == "constructor")
        {
            object_binding
                .non_enumerable_string_properties
                .push("constructor".to_string());
        }
        object_binding
    }

    pub(in crate::backend::direct_wasm) fn constructed_object_constructor_binding_for_shadow_argument(
        &self,
        argument_expression: &Expression,
    ) -> Option<LocalFunctionBinding> {
        self.resolve_constructed_object_constructor_binding(argument_expression)
            .or_else(|| {
                let Expression::Identifier(name) = argument_expression else {
                    return None;
                };
                let active_name = self
                    .resolve_current_local_binding(name)
                    .map(|(resolved_name, _)| resolved_name)
                    .unwrap_or_else(|| name.clone());
                let value = self
                    .state
                    .speculation
                    .static_semantics
                    .local_value_binding(&active_name)?;
                self.resolve_constructed_object_constructor_binding(value)
            })
    }

    pub(in crate::backend::direct_wasm) fn emit_user_function_parameter_object_shadow_writeback(
        &mut self,
        writebacks: &[(String, String, Option<ObjectValueBinding>)],
    ) -> DirectResult<()> {
        for (param_name, source_owner, _) in writebacks {
            self.emit_runtime_object_property_shadow_copy(param_name, source_owner)?;
        }
        Ok(())
    }

    fn clear_runtime_object_property_shadows_for_owner(
        &mut self,
        owner_name: &str,
        object_binding: &ObjectValueBinding,
    ) {
        for (property, _) in self.object_runtime_shadow_entries_from_binding(object_binding) {
            let binding =
                self.runtime_object_property_shadow_binding_by_property(owner_name, &property);
            let deleted_binding = self
                .runtime_object_property_shadow_deleted_binding_by_property(owner_name, &property);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(binding.value_index);
            self.push_i32_const(0);
            self.push_global_set(binding.present_index);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(deleted_binding.value_index);
            self.push_i32_const(0);
            self.push_global_set(deleted_binding.present_index);
        }
    }

    pub(in crate::backend::direct_wasm) fn clear_runtime_object_property_shadow_binding(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> bool {
        let binding = self.resolve_runtime_object_property_shadow_binding(object, property);
        let Some(deleted_binding) =
            self.resolve_runtime_object_property_shadow_deleted_binding(object, property)
        else {
            return false;
        };
        if let Some(binding) = binding {
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(binding.value_index);
            self.push_i32_const(0);
            self.push_global_set(binding.present_index);
        }
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_global_set(deleted_binding.value_index);
        self.push_i32_const(0);
        self.push_global_set(deleted_binding.present_index);
        true
    }

    /// Marks the deleted-shadow static metadata for a property as "not
    /// deleted". Stores that re-establish a previously deleted property emit
    /// a runtime clear of the deleted marker; the static metadata must follow
    /// so `runtime_object_property_shadow_deletion_is_statically_present`
    /// stops resolving the member to undefined.
    fn clear_runtime_object_property_shadow_deleted_static_metadata(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) {
        let Some(owner_name) =
            self.runtime_object_property_shadow_owner_name_for_expression(object)
        else {
            return;
        };
        let canonical_property = self.canonical_object_property_expression(property);
        let deleted_shadow_name =
            Self::runtime_object_property_deleted_shadow_name(&owner_name, &canonical_property);
        self.update_static_global_assignment_metadata(
            &deleted_shadow_name,
            &Expression::Number(0.0),
        );
    }

    pub(in crate::backend::direct_wasm) fn clear_runtime_object_property_shadow_deleted_binding(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> bool {
        let Some(binding) =
            self.resolve_runtime_object_property_shadow_deleted_binding(object, property)
        else {
            return false;
        };
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_global_set(binding.value_index);
        self.push_i32_const(0);
        self.push_global_set(binding.present_index);
        self.clear_runtime_object_property_shadow_deleted_static_metadata(object, property);
        true
    }

    pub(in crate::backend::direct_wasm) fn mark_runtime_object_property_shadow_deleted_binding(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> bool {
        let canonical_property = self.canonical_object_property_expression(property);
        let shadow_binding_name =
            self.runtime_object_property_shadow_binding_name_for_expression(object, property);
        let deleted_shadow_name = self
            .runtime_object_property_shadow_owner_name_for_expression(object)
            .map(|owner_name| {
                Self::runtime_object_property_deleted_shadow_name(&owner_name, &canonical_property)
            });
        let binding = self.resolve_runtime_object_property_shadow_binding(object, property);
        let Some(deleted_binding) =
            self.resolve_runtime_object_property_shadow_deleted_binding(object, property)
        else {
            return false;
        };
        if let Some(binding) = binding {
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(binding.value_index);
            self.push_i32_const(0);
            self.push_global_set(binding.present_index);
        }
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_global_set(deleted_binding.value_index);
        self.push_i32_const(1);
        self.push_global_set(deleted_binding.present_index);
        if let Some(deleted_shadow_name) = &deleted_shadow_name {
            self.backend
                .record_emitted_delete_shadow(deleted_shadow_name);
        }
        // A global-object property deletion must also set the dedicated
        // delete-sync flag when presence queries were compiled to read it:
        // the `this` shadow channel is saved/restored around user calls, so
        // the deleted-shadow pair alone cannot carry a deletion performed
        // inside an accessor or closure back to the caller's `in` checks.
        if let Expression::String(property_name) = &canonical_property
            && self
                .runtime_object_property_shadow_owner_name_for_expression(object)
                .is_some_and(|owner_name| owner_name == "this")
        {
            let sync_name = Self::global_object_property_delete_sync_binding_name(property_name);
            if self.backend.delete_shadow_was_emitted(&sync_name) {
                let sync_binding = self.ensure_implicit_global_binding(&sync_name);
                self.push_i32_const(JS_UNDEFINED_TAG);
                self.push_global_set(sync_binding.value_index);
                self.push_i32_const(1);
                self.push_global_set(sync_binding.present_index);
            }
        }
        if let Expression::String(property_name) = &canonical_property
            && let Some(function_name) = self.current_function_name()
            && !self.assigned_user_function_capture_originates_in_enclosing_local(
                function_name,
                property_name,
            )
            && let Some(hidden_name) = self.resolve_user_function_capture_hidden_name(property_name)
        {
            let deleted_marker_name =
                Self::capture_slot_member_source_deleted_binding_name(&hidden_name);
            let deleted_marker = self.ensure_implicit_global_binding(&deleted_marker_name);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(deleted_marker.value_index);
            self.push_i32_const(1);
            self.push_global_set(deleted_marker.present_index);
        }
        if let Some(shadow_binding_name) = shadow_binding_name {
            self.update_static_global_assignment_metadata(
                &shadow_binding_name,
                &Expression::Undefined,
            );
        }
        if let Some(deleted_shadow_name) = deleted_shadow_name {
            self.update_static_global_assignment_metadata(
                &deleted_shadow_name,
                &Expression::Undefined,
            );
        }
        if let Expression::Identifier(name) = object {
            let source_name = self
                .resolve_user_function_capture_hidden_name(name)
                .or_else(|| self.resolve_eval_local_function_hidden_name(name))
                .and_then(|hidden_name| self.resolve_capture_slot_source_binding_name(&hidden_name))
                .filter(|source_name| {
                    Self::capture_slot_member_source_key_parts(source_name).is_none()
                });
            if let Some(source_name) = source_name {
                let source_object = Expression::Identifier(source_name.clone());
                if let Some(source_binding) = self.resolve_runtime_object_property_shadow_binding(
                    &source_object,
                    &canonical_property,
                ) {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    self.push_global_set(source_binding.value_index);
                    self.push_i32_const(0);
                    self.push_global_set(source_binding.present_index);
                }
                if let Some(source_deleted_binding) = self
                    .resolve_runtime_object_property_shadow_deleted_binding(
                        &source_object,
                        &canonical_property,
                    )
                {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    self.push_global_set(source_deleted_binding.value_index);
                    self.push_i32_const(1);
                    self.push_global_set(source_deleted_binding.present_index);
                }
                if let Some(source_shadow_name) = self
                    .runtime_object_property_shadow_binding_name_for_expression(
                        &source_object,
                        &canonical_property,
                    )
                {
                    self.update_static_global_assignment_metadata(
                        &source_shadow_name,
                        &Expression::Undefined,
                    );
                }
                if let Some(source_owner_name) =
                    self.runtime_object_property_shadow_owner_name_for_expression(&source_object)
                {
                    let source_deleted_name = Self::runtime_object_property_deleted_shadow_name(
                        &source_owner_name,
                        &canonical_property,
                    );
                    self.update_static_global_assignment_metadata(
                        &source_deleted_name,
                        &Expression::Undefined,
                    );
                }
            }
        }
        true
    }

    pub(in crate::backend::direct_wasm) fn runtime_object_property_shadow_binding_name(
        owner_name: &str,
        property_name: &str,
    ) -> String {
        format!(
            "__ayy_object_property__{owner_name}__str__{}",
            Self::runtime_object_property_shadow_fragment(property_name)
        )
    }

    /// Names the implicit flag binding that records, at runtime, that a
    /// global-object property was deleted by an accessor observed from a
    /// strict closure store. The `this` shadow channel is saved and restored
    /// around user calls, so the deletion must be synced through a dedicated
    /// binding that global presence queries can read after the call returns.
    pub(in crate::backend::direct_wasm) fn global_object_property_delete_sync_binding_name(
        property_name: &str,
    ) -> String {
        format!(
            "__ayy_global_property_delete_sync__str__{}",
            Self::runtime_object_property_shadow_fragment(property_name)
        )
    }

    /// Follow the static value-binding alias chain from `name` while every
    /// hop lands on a synthetic class-related binding. When the chain
    /// terminates at a canonical class constructor channel, return the
    /// deepest chain entry that owns runtime shadow bindings, so every alias
    /// of a class object shares one shadow channel.
    pub(in crate::backend::direct_wasm) fn canonical_class_alias_shadow_owner(
        &self,
        name: &str,
    ) -> Option<String> {
        let is_class_channel_name = |candidate: &str| {
            candidate.starts_with("__ayy_class_ctor_") || candidate.starts_with("__ayy_class_expr_")
        };
        let is_chain_name = |candidate: &str| {
            is_class_channel_name(candidate)
                || candidate.starts_with("__ayy_local$")
                || candidate.starts_with("__ayy_scope$")
        };
        if is_class_channel_name(name) {
            return None;
        }
        let mut current = name.to_string();
        let mut deepest_owner_with_bindings = None;
        let mut reached_class_channel = false;
        for _ in 0..4 {
            let Some(Expression::Identifier(next)) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(&current)
                .or_else(|| self.global_value_binding(&current))
            else {
                break;
            };
            if next == &current || !is_chain_name(next) {
                break;
            }
            if self.runtime_object_property_shadow_owner_has_bindings(next) {
                deepest_owner_with_bindings = Some(next.clone());
            }
            if is_class_channel_name(next) {
                reached_class_channel = true;
                break;
            }
            current = next.clone();
        }
        if !reached_class_channel {
            return None;
        }
        deepest_owner_with_bindings.filter(|owner| owner != name)
    }

    pub(in crate::backend::direct_wasm) fn runtime_object_property_shadow_owner_name_for_identifier(
        &self,
        name: &str,
    ) -> Option<String> {
        // Value bindings can alias each other in cycles (`a` recorded as `b`
        // while `b` is recorded as `a`, for example through with-scope shadow
        // copies), so bound the recursion depth.
        let _guard = RuntimeShadowOwnerExpressionGuard::enter()?;
        let identifier_expression = Expression::Identifier(name.to_string());
        let trace_runtime_shadows = crate::ayy_env_flag!("AYY_TRACE_RUNTIME_SHADOWS");
        if name.starts_with("__ayy_target_object_")
            && let Some(source) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
                .or_else(|| self.global_value_binding(name))
            && let Some(owner) = self.runtime_assignment_target_object_shadow_owner(source)
        {
            return Some(owner);
        }
        if name == "this" {
            if self
                .current_user_function()
                .is_some_and(|function| function.lexical_this)
                && let Some(hidden_name) = self.resolve_user_function_capture_hidden_name("this")
            {
                return Some(hidden_name);
            }
            return Some("this".to_string());
        }
        if self.is_unshadowed_builtin_identifier(name)
            && matches!(
                builtin_identifier_kind(name),
                Some(StaticValueKind::Object | StaticValueKind::Function)
            )
        {
            return Some(name.to_string());
        }
        // Class bindings and their scoped/class-local aliases all denote the
        // same object identity; route every alias whose static value chain
        // reaches the canonical class constructor channel through the deepest
        // chain entry that owns shadow bindings, so private member and static
        // property state stays on a single shadow channel regardless of which
        // alias performed the access.
        if let Some(class_owner) = self.canonical_class_alias_shadow_owner(name) {
            if trace_runtime_shadows {
                eprintln!("runtime_shadow_owner_class_canonical name={name} owner={class_owner}");
            }
            return Some(class_owner);
        }
        if self.contains_user_function(name) {
            return Some(name.to_string());
        }
        if self.resolve_user_function_by_binding_name(name).is_some() {
            return Some(name.to_string());
        }
        if let Some(source_name) = scoped_binding_source_name(name)
            && (self.runtime_object_property_shadow_owner_has_bindings(source_name)
                || self.backend.global_has_binding(source_name)
                || self.backend.global_has_lexical_binding(source_name)
                || self.backend.global_has_implicit_binding(source_name)
                || self.global_value_binding(source_name).is_some()
                || self.contains_user_function(source_name)
                || self
                    .resolve_user_function_by_binding_name(source_name)
                    .is_some())
        {
            return Some(source_name.to_string());
        }
        if name.starts_with("__ayy_for_in_target_")
            && let Some(Expression::Identifier(source_name)) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
            && self
                .runtime_object_property_shadow_owner_name_for_identifier(source_name)
                .is_some()
        {
            return Some(source_name.clone());
        }
        if let Some(hidden_name) = self.resolve_user_function_capture_hidden_name(name) {
            // Captured class bindings alias the canonical class constructor
            // channel; capture-prep skips the runtime shadow copy for them, so
            // private member state only lives on the canonical channel. Route
            // reads and writes through that channel instead of the (never
            // populated) capture channel.
            let class_owner_candidates = [
                self.state
                    .speculation
                    .static_semantics
                    .local_value_binding(&hidden_name)
                    .or_else(|| self.global_value_binding(&hidden_name))
                    .cloned(),
                self.resolve_bound_alias_expression(&identifier_expression),
            ];
            let class_owner = class_owner_candidates
                .into_iter()
                .flatten()
                .find_map(|candidate| match candidate {
                    Expression::Identifier(source_name)
                        if source_name != hidden_name
                            && (source_name.starts_with("__ayy_class_ctor_")
                                || source_name.starts_with("__ayy_class_expr_"))
                            && self.runtime_object_property_shadow_owner_has_bindings(
                                &source_name,
                            ) =>
                    {
                        Some(source_name)
                    }
                    _ => None,
                });
            if let Some(class_owner) = class_owner {
                return Some(class_owner);
            }
            return Some(hidden_name);
        }
        if let Some(hidden_name) = self.resolve_eval_local_function_hidden_name(name) {
            return Some(hidden_name);
        }
        if name.starts_with("__ayy_member_object__")
            && self.runtime_object_property_shadow_owner_has_bindings(name)
        {
            return Some(name.to_string());
        }
        if self.hidden_implicit_global_binding(name).is_some()
            && self.runtime_object_property_shadow_owner_has_bindings(name)
        {
            return Some(name.to_string());
        }
        if let Some(Expression::Identifier(source_name)) = self
            .state
            .speculation
            .static_semantics
            .local_value_binding(name)
            .or_else(|| self.global_value_binding(name))
            && source_name != name
            && let Some(source_owner) =
                self.runtime_object_property_shadow_owner_name_for_identifier(source_name)
        {
            return Some(source_owner);
        }
        if let Some((resolved_name, _)) = self.resolve_current_local_binding(name) {
            if resolved_name != name
                && (self
                    .state
                    .speculation
                    .static_semantics
                    .has_local_object_binding(&resolved_name)
                    || self.runtime_object_property_shadow_owner_has_bindings(&resolved_name))
            {
                return Some(resolved_name);
            }
            return Some(name.to_string());
        }
        if (self.backend.global_has_binding(name)
            || self.backend.global_has_lexical_binding(name)
            || self.backend.global_has_implicit_binding(name))
            && self.runtime_object_property_shadow_owner_has_bindings(name)
        {
            return Some(name.to_string());
        }
        if self
            .state
            .speculation
            .static_semantics
            .has_local_object_binding(name)
        {
            return Some(name.to_string());
        }
        if self.current_function_name().is_some_and(|function_name| {
            self.backend
                .function_registry
                .parameter_bindings_for(function_name)
                .object_bindings
                .contains_key(name)
                || self
                    .user_function(function_name)
                    .is_some_and(|function| function.params.iter().any(|param| param == name))
        }) {
            return Some(name.to_string());
        }
        if self.identifier_static_value_is_call_expression(name)
            && !self.runtime_object_property_shadow_owner_has_bindings(name)
            && !self
                .state
                .speculation
                .static_semantics
                .has_local_object_binding(name)
            && self.backend.global_object_binding(name).is_none()
        {
            return None;
        }
        let resolved_owner = ((self.backend.global_has_binding(name)
            || self.backend.global_has_lexical_binding(name)
            || self.backend.global_function_binding(name).is_some()
            || self.backend.global_has_implicit_binding(name))
            && self.backend.global_object_binding(name).is_some())
        .then(|| name.to_string())
        .or_else(|| {
            (self.backend.global_has_implicit_binding(name)
                && self.backend.global_object_binding(name).is_some())
            .then(|| name.to_string())
        })
        .or_else(|| {
            self.resolve_bound_alias_expression(&identifier_expression)
                .filter(|resolved| !static_expression_matches(resolved, &identifier_expression))
                .or_else(|| {
                    self.state
                        .speculation
                        .static_semantics
                        .local_value_binding(name)
                        .cloned()
                })
                .or_else(|| self.global_value_binding(name).cloned())
                .filter(|resolved| !static_expression_matches(resolved, &identifier_expression))
                .filter(|resolved| {
                    !matches!(resolved, Expression::Call { .. })
                        && !self.expression_is_user_function_call_with_source_loop(resolved)
                })
                .and_then(|resolved| {
                    self.runtime_object_property_shadow_owner_name_for_expression(&resolved)
                        .or_else(|| {
                            (expression_may_evaluate_to_runtime_shadow_owner(&resolved)
                                && (matches!(
                                    self.infer_value_kind(&resolved),
                                    Some(StaticValueKind::Object | StaticValueKind::Function)
                                ) || self
                                    .resolve_object_binding_from_expression(&resolved)
                                    .is_some()))
                            .then(|| name.to_string())
                        })
                })
        });
        if trace_runtime_shadows {
            eprintln!(
                "runtime_shadow_owner identifier={name} fn={:?} local_object={} local_value={:?} global_object={} global_value={:?} alias={:?} resolved_owner={resolved_owner:?}",
                self.current_function_name(),
                self.state
                    .speculation
                    .static_semantics
                    .has_local_object_binding(name),
                self.state
                    .speculation
                    .static_semantics
                    .local_value_binding(name)
                    .cloned(),
                self.backend.global_object_binding(name).is_some(),
                self.global_value_binding(name).cloned(),
                self.resolve_bound_alias_expression(&identifier_expression)
                    .filter(|resolved| !static_expression_matches(
                        resolved,
                        &identifier_expression
                    )),
            );
        }
        resolved_owner
    }

    pub(in crate::backend::direct_wasm) fn identifier_static_value_is_call_expression(
        &self,
        name: &str,
    ) -> bool {
        let mut current_name = name;
        for _ in 0..16 {
            let Some(value) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(current_name)
                .or_else(|| self.global_value_binding(current_name))
            else {
                return false;
            };
            match value {
                Expression::Call { .. } => return true,
                Expression::Identifier(alias) if alias != current_name => {
                    current_name = alias;
                }
                _ => return false,
            }
        }
        false
    }

    pub(in crate::backend::direct_wasm) fn runtime_object_reference_alias_owner_names(
        &self,
        owner_name: &str,
    ) -> Vec<String> {
        let Some(owner_key) = self.reference_identity_key_for_identifier(owner_name) else {
            return Vec::new();
        };

        let mut candidates = self
            .state
            .speculation
            .static_semantics
            .values
            .local_value_bindings
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        candidates.extend(
            self.backend
                .global_semantics
                .values
                .value_bindings
                .keys()
                .cloned(),
        );
        candidates.extend(
            self.backend
                .shared_global_semantics
                .values
                .value_bindings
                .keys()
                .cloned(),
        );

        let mut aliases = Vec::new();
        for candidate in candidates {
            if candidate == owner_name || aliases.iter().any(|alias| alias == &candidate) {
                continue;
            }
            if self
                .reference_identity_key_for_identifier(&candidate)
                .is_some_and(|candidate_key| candidate_key == owner_key)
            {
                aliases.push(candidate);
            }
        }
        aliases
    }

    fn runtime_assignment_target_object_shadow_owner(
        &self,
        expression: &Expression,
    ) -> Option<String> {
        match expression {
            Expression::Identifier(name) => {
                self.runtime_object_property_shadow_owner_name_for_identifier(name)
            }
            Expression::This => {
                self.runtime_object_property_shadow_owner_name_for_identifier("this")
            }
            Expression::Member { object, property } => {
                let property = self
                    .resolve_property_key_expression(property)
                    .unwrap_or_else(|| self.materialize_static_expression(property));
                let object_owner = self
                    .runtime_assignment_target_object_shadow_owner(object)
                    .or_else(|| {
                        self.runtime_object_property_shadow_owner_name_for_expression(object)
                    })?;
                let materialized = self.materialize_static_expression(expression);
                let materialized_is_object = matches!(
                    self.infer_value_kind(&materialized),
                    Some(StaticValueKind::Object | StaticValueKind::Function)
                ) || self
                    .resolve_object_binding_from_expression(&materialized)
                    .is_some()
                    || self
                        .resolve_array_binding_from_expression(&materialized)
                        .is_some();
                materialized_is_object.then(|| {
                    Self::runtime_object_member_shadow_owner_name(&object_owner, &property)
                })
            }
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn runtime_object_property_shadow_owner_name_for_expression(
        &self,
        expression: &Expression,
    ) -> Option<String> {
        // Property values resolved from object/shadow bindings can reference
        // expressions that resolve back through this same lookup (for example
        // with-scope shadows whose recorded value is the member expression
        // itself), so guard against unbounded self-recursion.
        let _guard = RuntimeShadowOwnerExpressionGuard::enter()?;
        match expression {
            Expression::Identifier(name) => {
                self.runtime_object_property_shadow_owner_name_for_identifier(name)
            }
            Expression::This => {
                self.runtime_object_property_shadow_owner_name_for_identifier("this")
            }
            Expression::Member { object, property }
                if self.is_direct_arguments_object(object)
                    && argument_index_from_expression(
                        &self.canonical_object_property_expression(property),
                    )
                    .is_some() =>
            {
                let index = argument_index_from_expression(
                    &self.canonical_object_property_expression(property),
                )?;
                let function_name = self.current_function_name()?;
                Some(Self::user_function_arguments_slot_object_shadow_owner_name(
                    function_name,
                    index,
                ))
            }
            Expression::Member { object, property } => {
                let property = self
                    .resolve_property_key_expression(property)
                    .unwrap_or_else(|| self.materialize_static_expression(property));
                let direct_object_owner = match object.as_ref() {
                    Expression::Identifier(name) => Some(name.as_str()),
                    Expression::This => Some("this"),
                    _ => None,
                };
                if let Some(object_owner) = direct_object_owner {
                    let member_owner =
                        Self::runtime_object_member_shadow_owner_name(object_owner, &property);
                    if let Some(alias_owner) = self
                        .runtime_object_property_shadow_static_alias_owner_for_owner_property(
                            object_owner,
                            &property,
                            &member_owner,
                        )
                    {
                        return Some(alias_owner);
                    }
                    if self.runtime_object_property_shadow_owner_has_bindings(&member_owner) {
                        return Some(member_owner);
                    }
                }
                if let Some(object_owner) =
                    self.runtime_object_property_shadow_owner_name_for_expression(object)
                {
                    let member_owner =
                        Self::runtime_object_member_shadow_owner_name(&object_owner, &property);
                    if let Some(alias_owner) = self
                        .runtime_object_property_shadow_static_alias_owner_for_owner_property(
                            &object_owner,
                            &property,
                            &member_owner,
                        )
                    {
                        return Some(alias_owner);
                    }
                    if self.runtime_object_property_shadow_owner_has_bindings(&member_owner) {
                        return Some(member_owner);
                    }
                    if self.runtime_object_property_shadow_binding_exists_for_owner_property(
                        &object_owner,
                        &property,
                    ) {
                        return Some(member_owner);
                    }
                }
                let mut values = Vec::new();
                if let Some(value) = self
                    .resolve_object_binding_from_expression(object)
                    .and_then(|object_binding| {
                        self.resolve_object_binding_property_value(&object_binding, &property)
                    })
                {
                    values.push(value);
                }
                if let Some(shadow_binding_name) = self
                    .runtime_object_property_shadow_binding_name_for_expression(object, &property)
                    && let Some(value) = self
                        .global_value_binding(&shadow_binding_name)
                        .cloned()
                        .or_else(|| {
                            self.backend
                                .shared_global_semantics
                                .values
                                .value_bindings
                                .get(&shadow_binding_name)
                                .cloned()
                        })
                {
                    values.push(value);
                }
                values
                    .into_iter()
                    .filter(|value| {
                        !matches!(value, Expression::Call { .. })
                            && !self.expression_is_user_function_call_with_source_loop(value)
                    })
                    .find_map(|value| {
                        self.runtime_object_property_shadow_owner_name_for_expression(&value)
                            .or_else(|| {
                                let materialized = self.materialize_static_expression(&value);
                                (!static_expression_matches(&materialized, &value))
                                .then(|| {
                                    self.runtime_object_property_shadow_owner_name_for_expression(
                                        &materialized,
                                    )
                                })
                                .flatten()
                            })
                    })
            }
            _ => None,
        }
    }

    fn runtime_object_property_shadow_binding_exists_for_owner_property(
        &self,
        owner_name: &str,
        property: &Expression,
    ) -> bool {
        let shadow_key = Self::runtime_object_property_shadow_key(property);
        let shadow_name = format!("__ayy_object_property__{owner_name}__{shadow_key}");
        let deleted_name = Self::runtime_object_property_deleted_shadow_name(owner_name, property);
        self.global_has_implicit_binding(&shadow_name)
            || self.global_has_implicit_binding(&deleted_name)
            || self
                .backend
                .shared_global_semantics
                .global_names()
                .implicit_bindings
                .contains_key(&shadow_name)
            || self
                .backend
                .shared_global_semantics
                .global_names()
                .implicit_bindings
                .contains_key(&deleted_name)
            || self.backend.delete_shadow_was_emitted(&deleted_name)
    }

    fn runtime_member_value_shadow_source_owners(
        &self,
        source_owner: &str,
        property: &Expression,
        fallback_value: Option<&Expression>,
    ) -> Vec<String> {
        let mut owners = Vec::new();
        let source_member_owner =
            Self::runtime_object_member_shadow_owner_name(source_owner, property);
        if self.runtime_object_property_shadow_owner_has_bindings(&source_member_owner) {
            owners.push(source_member_owner);
        }
        if let Some(fallback_value) = fallback_value {
            if matches!(fallback_value, Expression::Call { .. })
                || self.expression_is_user_function_call_with_source_loop(fallback_value)
            {
                return owners;
            }
            if let Some(owner) =
                self.runtime_object_property_shadow_owner_name_for_expression(fallback_value)
            {
                if !owners.iter().any(|existing| existing == &owner) {
                    owners.push(owner);
                }
            }
            let materialized = self.materialize_static_expression(fallback_value);
            if !static_expression_matches(&materialized, fallback_value)
                && let Some(owner) =
                    self.runtime_object_property_shadow_owner_name_for_expression(&materialized)
                && !owners.iter().any(|existing| existing == &owner)
            {
                owners.push(owner);
            }
        }
        owners
    }

    fn predeclare_runtime_object_property_shadow_copy_target_bindings(
        &mut self,
        source_owner: &str,
        target_owner: &str,
    ) {
        let mut entries = self.runtime_object_property_shadow_copy_entries(source_owner);
        self.append_target_private_runtime_shadow_copy_entries(
            source_owner,
            target_owner,
            &mut entries,
        );
        for (property, _) in entries {
            self.runtime_object_property_shadow_binding_by_property(target_owner, &property);
            self.runtime_object_property_shadow_deleted_binding_by_property(
                target_owner,
                &property,
            );
        }

        let source_prefix = format!("__ayy_object_property__{source_owner}__");
        let source_deleted_prefix = format!("__ayy_object_property_deleted__{source_owner}__");
        let suffixes = self
            .active_runtime_object_shadow_names_with_prefix(&source_prefix)
            .into_iter()
            .chain(
                self.active_runtime_object_shadow_names_with_prefix(&source_deleted_prefix)
                    .into_iter(),
            )
            .filter_map(|(_, name)| {
                name.strip_prefix(&source_prefix)
                    .or_else(|| name.strip_prefix(&source_deleted_prefix))
                    .map(str::to_string)
            })
            .collect::<HashSet<_>>();
        for suffix in suffixes {
            self.ensure_implicit_global_binding(&format!(
                "__ayy_object_property__{target_owner}__{suffix}"
            ));
            self.ensure_implicit_global_binding(&format!(
                "__ayy_object_property_deleted__{target_owner}__{suffix}"
            ));
        }
    }

    fn predeclare_runtime_member_value_shadow_targets(
        &mut self,
        source_owner: &str,
        target_owner: &str,
        property: &Expression,
        fallback_value: Option<&Expression>,
    ) -> String {
        let member_owner = Self::runtime_object_member_shadow_owner_name(target_owner, property);
        let source_owners =
            self.runtime_member_value_shadow_source_owners(source_owner, property, fallback_value);
        for source_owner in source_owners {
            self.predeclare_runtime_object_property_shadow_copy_target_bindings(
                &source_owner,
                &member_owner,
            );
        }
        member_owner
    }

    fn emit_clear_runtime_member_value_shadow(
        &mut self,
        source_owner: &str,
        target_owner: &str,
        property: &Expression,
        fallback_value: Option<&Expression>,
    ) {
        if Self::runtime_shadow_property_is_private(property) {
            return;
        }
        let member_owner = self.predeclare_runtime_member_value_shadow_targets(
            source_owner,
            target_owner,
            property,
            fallback_value,
        );
        self.clear_runtime_object_property_shadow_prefix(&member_owner);
    }

    fn emit_refresh_runtime_member_value_shadow(
        &mut self,
        source_owner: &str,
        target_owner: &str,
        property: &Expression,
        fallback_value: Option<&Expression>,
    ) -> DirectResult<()> {
        if Self::runtime_shadow_property_is_private(property) {
            return Ok(());
        }
        let member_owner = Self::runtime_object_member_shadow_owner_name(target_owner, property);
        if fallback_value
            .and_then(|value| self.runtime_shadow_static_value_owner(value))
            .is_some_and(|owner| {
                owner == source_owner || owner == target_owner || owner == member_owner
            })
        {
            self.clear_runtime_object_property_shadow_prefix(&member_owner);
            self.clear_runtime_object_property_shadow_static_metadata_prefix(&member_owner);
            return Ok(());
        }
        let member_owner = self.predeclare_runtime_member_value_shadow_targets(
            source_owner,
            target_owner,
            property,
            fallback_value,
        );
        let source_owners =
            self.runtime_member_value_shadow_source_owners(source_owner, property, fallback_value);
        self.clear_runtime_object_property_shadow_prefix(&member_owner);
        for source_owner in source_owners {
            self.emit_runtime_object_property_shadow_copy(&source_owner, &member_owner)?;
        }
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_refresh_runtime_member_value_shadow_from_store(
        &mut self,
        target_object: &Expression,
        property: &Expression,
        value_expression: &Expression,
    ) -> DirectResult<()> {
        let property = self
            .resolve_property_key_expression(property)
            .unwrap_or_else(|| self.materialize_static_expression(property));
        let property = self.canonical_runtime_shadow_property_expression(&property);
        if Self::runtime_shadow_property_is_private(&property) {
            return Ok(());
        }

        let Some(target_owner) =
            self.runtime_object_property_shadow_owner_name_for_expression(target_object)
        else {
            return Ok(());
        };
        let member_owner = Self::runtime_object_member_shadow_owner_name(&target_owner, &property);
        let fallback_value = self.reference_preserving_static_value_expression(value_expression);
        let shadow_key = Self::runtime_object_property_shadow_key(&property);
        let shadow_binding_name = format!("__ayy_object_property__{target_owner}__{shadow_key}");
        self.ensure_implicit_global_binding(&shadow_binding_name);
        if !Self::expression_is_runtime_object_property_shadow_identifier(&fallback_value)
            && self.runtime_shadow_fallback_references_readable_bindings(&fallback_value)
        {
            self.update_static_global_assignment_metadata(&shadow_binding_name, &fallback_value);
        }
        if self
            .runtime_shadow_static_value_owner(&fallback_value)
            .is_some_and(|owner| owner == target_owner.as_str())
        {
            self.clear_runtime_object_property_shadow_prefix(&member_owner);
            self.clear_runtime_object_property_shadow_static_metadata_prefix(&member_owner);
            return Ok(());
        }
        let mut source_owners = Vec::new();
        if !matches!(fallback_value, Expression::Call { .. })
            && !self.expression_is_user_function_call_with_source_loop(&fallback_value)
        {
            if let Some(owner) =
                self.runtime_object_property_shadow_owner_name_for_expression(&fallback_value)
            {
                source_owners.push(owner);
            }
            let materialized = self.materialize_static_expression(&fallback_value);
            if !static_expression_matches(&materialized, &fallback_value)
                && let Some(owner) =
                    self.runtime_object_property_shadow_owner_name_for_expression(&materialized)
                && !source_owners.iter().any(|existing| existing == &owner)
            {
                source_owners.push(owner);
            }
        }

        if source_owners.len() == 1 && source_owners[0] == member_owner {
            return Ok(());
        }

        let mut sibling_alias_owners = self.runtime_member_shadow_sibling_alias_owners(
            target_object,
            &target_owner,
            &property,
        );
        sibling_alias_owners.sort_by(|left, right| match (&left.guard, &right.guard) {
            (Some(left_guard), Some(right_guard)) => right_guard.depth.cmp(&left_guard.depth),
            (Some(_), None) => std::cmp::Ordering::Greater,
            (None, Some(_)) => std::cmp::Ordering::Less,
            (None, None) => std::cmp::Ordering::Equal,
        });
        let source_object_bindings = source_owners
            .iter()
            .filter_map(|source_owner| {
                self.resolve_runtime_shadow_object_binding(source_owner)
                    .map(|object_binding| (source_owner.clone(), object_binding))
            })
            .collect::<Vec<_>>();
        let direct_object_binding = source_object_bindings
            .is_empty()
            .then(|| {
                self.resolve_object_binding_from_expression(&fallback_value)
                    .or_else(|| {
                        let materialized = self.materialize_static_expression(&fallback_value);
                        (!static_expression_matches(&materialized, &fallback_value))
                            .then(|| self.resolve_object_binding_from_expression(&materialized))
                            .flatten()
                    })
            })
            .flatten();

        if crate::ayy_env_flag!("AYY_TRACE_RUNTIME_SHADOWS") {
            eprintln!(
                "runtime_member_shadow_store target_owner={target_owner} property={property:?} member_owner={member_owner} value={fallback_value:?} sources={source_owners:?} direct_object={}",
                direct_object_binding.is_some()
            );
        }

        self.clear_runtime_object_property_shadow_prefix(&member_owner);
        self.clear_runtime_object_property_shadow_static_metadata_prefix(&member_owner);

        for source_owner in &source_owners {
            self.emit_runtime_object_property_shadow_copy(source_owner, &member_owner)?;
        }
        for (_, object_binding) in &source_object_bindings {
            self.sync_runtime_object_property_shadow_static_metadata_from_binding(
                &member_owner,
                object_binding,
            );
        }
        if let Some(object_binding) = direct_object_binding.as_ref() {
            self.emit_runtime_object_property_shadow_seed_from_binding(
                &member_owner,
                object_binding,
            )?;
            self.sync_runtime_object_property_shadow_static_metadata_from_binding(
                &member_owner,
                object_binding,
            );
        }
        for alias in sibling_alias_owners {
            let alias_owner = alias.owner;
            if crate::ayy_env_flag!("AYY_TRACE_RUNTIME_SHADOWS") {
                eprintln!(
                    "runtime_member_shadow_alias_update source={target_owner} alias={alias_owner} property={property:?}"
                );
            }
            if let Some(guard) = alias.guard.as_ref() {
                self.emit_guarded_runtime_member_shadow_alias_property_copy(
                    &target_owner,
                    &alias_owner,
                    &property,
                    guard,
                )?;
            } else {
                self.emit_runtime_object_property_shadow_property_copy_between_exact_owners(
                    &target_owner,
                    &alias_owner,
                    &property,
                )?;
            }
            let alias_member_owner =
                Self::runtime_object_member_shadow_owner_name(&alias_owner, &property);
            let alias_shadow_binding_name =
                format!("__ayy_object_property__{alias_owner}__{shadow_key}");
            let alias_static_value = if self
                .resolve_runtime_shadow_object_binding(&member_owner)
                .is_some()
                && expression_may_evaluate_to_runtime_shadow_owner(&fallback_value)
            {
                Expression::Identifier(alias_member_owner.clone())
            } else {
                fallback_value.clone()
            };
            let alias_static_value_is_member_owner = matches!(
                &alias_static_value,
                Expression::Identifier(name) if name.starts_with("__ayy_member_object__")
            );
            if !Self::expression_is_runtime_object_property_shadow_identifier(&alias_static_value)
                && (alias_static_value_is_member_owner
                    || self
                        .runtime_shadow_fallback_references_readable_bindings(&alias_static_value))
            {
                self.ensure_implicit_global_binding(&alias_shadow_binding_name);
                self.update_static_global_assignment_metadata(
                    &alias_shadow_binding_name,
                    &alias_static_value,
                );
            }
            if let Some(updated_binding) = self.resolve_runtime_shadow_object_binding(&member_owner)
            {
                self.sync_runtime_object_property_shadow_static_metadata_from_binding(
                    &alias_member_owner,
                    &updated_binding,
                );
            }
        }

        Ok(())
    }

    fn runtime_member_shadow_sibling_alias_owners(
        &self,
        target_object: &Expression,
        target_owner: &str,
        assigned_property: &Expression,
    ) -> Vec<RuntimeMemberShadowAliasOwner> {
        let Expression::Member {
            object: parent_object,
            property: target_property,
        } = target_object
        else {
            return Vec::new();
        };
        let Some(parent_owner) =
            self.runtime_object_property_shadow_owner_name_for_expression(parent_object)
        else {
            return Vec::new();
        };
        let target_property = self
            .resolve_property_key_expression(target_property)
            .unwrap_or_else(|| self.materialize_static_expression(target_property));
        let target_property = self.canonical_runtime_shadow_property_expression(&target_property);
        let Some(target_binding) = self.resolve_runtime_shadow_object_binding(target_owner) else {
            return Vec::new();
        };
        if self
            .object_runtime_shadow_entries_from_binding(&target_binding)
            .is_empty()
        {
            return Vec::new();
        }
        let target_parent_value = self
            .runtime_object_property_shadow_static_value_for_owner(&parent_owner, &target_property);
        if crate::ayy_env_flag!("AYY_TRACE_RUNTIME_SHADOWS") {
            eprintln!(
                "runtime_member_shadow_alias_scan parent={parent_owner} target_owner={target_owner} target_property={target_property:?} target_parent_value={target_parent_value:?}"
            );
        }

        let mut aliases: Vec<RuntimeMemberShadowAliasOwner> = Vec::new();
        for (candidate_property, _) in
            self.runtime_object_property_shadow_copy_entries(&parent_owner)
        {
            let candidate_property =
                self.canonical_runtime_shadow_property_expression(&candidate_property);
            if static_expression_matches(&candidate_property, &target_property) {
                continue;
            }
            let candidate_owner =
                Self::runtime_object_member_shadow_owner_name(&parent_owner, &candidate_property);
            if candidate_owner == target_owner
                || aliases.iter().any(|alias| alias.owner == candidate_owner)
                || !self.runtime_object_property_shadow_owner_has_bindings(&candidate_owner)
            {
                continue;
            }
            let structurally_aliases = self
                .resolve_runtime_shadow_object_binding(&candidate_owner)
                .is_some_and(|candidate_binding| {
                    candidate_binding == target_binding
                        || (parent_owner.starts_with("__ayy_inline_param_")
                            && self.runtime_shadow_object_binding_entries_match_ignoring_property(
                                &candidate_binding,
                                &target_binding,
                                Some(assigned_property),
                            ))
                });
            let null_tail_aliases = self
                .runtime_member_shadow_null_next_tail_aliases(&candidate_owner, assigned_property);
            let value_aliases = target_parent_value
                .as_ref()
                .zip(
                    self.runtime_object_property_shadow_static_value_for_owner(
                        &parent_owner,
                        &candidate_property,
                    )
                    .as_ref(),
                )
                .is_some_and(|(target_value, candidate_value)| {
                    static_expression_matches(target_value, candidate_value)
                });
            if crate::ayy_env_flag!("AYY_TRACE_RUNTIME_SHADOWS") {
                let candidate_value = self.runtime_object_property_shadow_static_value_for_owner(
                    &parent_owner,
                    &candidate_property,
                );
                eprintln!(
                    "runtime_member_shadow_alias_candidate parent={parent_owner} candidate_owner={candidate_owner} candidate_property={candidate_property:?} structural={structurally_aliases} value={value_aliases} null_tail={null_tail_aliases} candidate_value={candidate_value:?}"
                );
            }
            if structurally_aliases || value_aliases || null_tail_aliases {
                let guard =
                    (!structurally_aliases && !value_aliases && null_tail_aliases).then(|| {
                        RuntimeMemberShadowAliasGuard {
                            parent_owner: parent_owner.clone(),
                            parent_property: candidate_property.clone(),
                            assigned_property: assigned_property.clone(),
                            depth: 1,
                        }
                    });
                aliases.push(RuntimeMemberShadowAliasOwner {
                    owner: candidate_owner,
                    guard,
                });
            }
        }
        let mut visited_alias_owners = HashSet::new();
        self.collect_nested_runtime_member_shadow_alias_owners(
            &parent_owner,
            &target_property,
            assigned_property,
            target_owner,
            &target_binding,
            target_parent_value.as_ref(),
            &mut aliases,
            &mut visited_alias_owners,
            16,
        );
        aliases
    }

    fn collect_nested_runtime_member_shadow_alias_owners(
        &self,
        owner_name: &str,
        root_target_property: &Expression,
        assigned_property: &Expression,
        target_owner: &str,
        target_binding: &ObjectValueBinding,
        target_parent_value: Option<&Expression>,
        aliases: &mut Vec<RuntimeMemberShadowAliasOwner>,
        visited: &mut HashSet<String>,
        remaining_depth: usize,
    ) {
        if remaining_depth == 0 || !visited.insert(owner_name.to_string()) {
            return;
        }

        for (candidate_property, _) in self.runtime_object_property_shadow_copy_entries(owner_name)
        {
            let candidate_property =
                self.canonical_runtime_shadow_property_expression(&candidate_property);
            let candidate_owner =
                Self::runtime_object_member_shadow_owner_name(owner_name, &candidate_property);
            let candidate_owner_has_bindings =
                self.runtime_object_property_shadow_owner_has_bindings(&candidate_owner);
            if owner_name == target_owner || candidate_owner == target_owner {
                continue;
            }
            if owner_name != target_owner
                && !(visited.len() == 1
                    && static_expression_matches(&candidate_property, root_target_property))
                && candidate_owner_has_bindings
                && !aliases.iter().any(|alias| alias.owner == candidate_owner)
            {
                let candidate_binding =
                    self.resolve_runtime_shadow_object_binding(&candidate_owner);
                let candidate_value = self.runtime_object_property_shadow_static_value_for_owner(
                    owner_name,
                    &candidate_property,
                );
                let candidate_has_object_entries =
                    candidate_binding.as_ref().is_some_and(|binding| {
                        !self
                            .object_runtime_shadow_entries_from_binding(binding)
                            .is_empty()
                    });
                let candidate_value_is_known_non_object = !candidate_has_object_entries
                    && matches!(
                        candidate_value,
                        Some(Expression::Null | Expression::Undefined)
                    );
                let structurally_aliases = !candidate_value_is_known_non_object
                    && candidate_binding.as_ref().is_some_and(|binding| {
                        binding == target_binding
                            || self.runtime_shadow_object_binding_entries_match_ignoring_property(
                                binding,
                                target_binding,
                                Some(assigned_property),
                            )
                    });
                let alias_depth = 17 - remaining_depth;
                let null_tail_aliases = alias_depth
                    <= RUNTIME_MEMBER_SHADOW_NULL_TAIL_ALIAS_DEPTH_LIMIT
                    && self.runtime_member_shadow_null_next_tail_aliases(
                        &candidate_owner,
                        assigned_property,
                    );
                let value_aliases = target_parent_value
                    .zip(candidate_value.as_ref())
                    .is_some_and(|(target_value, candidate_value)| {
                        static_expression_matches(target_value, candidate_value)
                    })
                    || candidate_value.as_ref().is_some_and(|candidate_value| {
                        matches!(candidate_value, Expression::Identifier(name) if name == target_owner)
                    });
                if crate::ayy_env_flag!("AYY_TRACE_RUNTIME_SHADOWS") {
                    eprintln!(
                        "runtime_member_shadow_nested_alias_candidate owner={owner_name} candidate_owner={candidate_owner} candidate_property={candidate_property:?} structural={structurally_aliases} value={value_aliases} null_tail={null_tail_aliases} candidate_value={candidate_value:?}"
                    );
                }
                if structurally_aliases || value_aliases || null_tail_aliases {
                    let guard = (!structurally_aliases && !value_aliases && null_tail_aliases)
                        .then(|| RuntimeMemberShadowAliasGuard {
                            parent_owner: owner_name.to_string(),
                            parent_property: candidate_property.clone(),
                            assigned_property: assigned_property.clone(),
                            depth: alias_depth,
                        });
                    aliases.push(RuntimeMemberShadowAliasOwner {
                        owner: candidate_owner.clone(),
                        guard,
                    });
                }
            }
            if candidate_owner_has_bindings {
                self.collect_nested_runtime_member_shadow_alias_owners(
                    &candidate_owner,
                    root_target_property,
                    assigned_property,
                    target_owner,
                    target_binding,
                    target_parent_value,
                    aliases,
                    visited,
                    remaining_depth - 1,
                );
            }
        }
    }

    fn runtime_member_shadow_null_next_tail_aliases(
        &self,
        candidate_owner: &str,
        assigned_property: &Expression,
    ) -> bool {
        if !matches!(assigned_property, Expression::String(property_name) if property_name == "next")
        {
            return false;
        }
        match self.runtime_object_property_shadow_static_value_for_owner(
            candidate_owner,
            assigned_property,
        ) {
            Some(Expression::Null | Expression::Undefined) => true,
            Some(Expression::Identifier(owner_name))
                if owner_name.starts_with("__ayy_member_object__") =>
            {
                self.runtime_member_shadow_owner_has_no_concrete_static_entries(&owner_name)
            }
            None if candidate_owner.starts_with("__ayy_member_object__") => {
                self.runtime_member_shadow_owner_has_no_concrete_static_entries(candidate_owner)
            }
            _ => false,
        }
    }

    fn runtime_member_shadow_owner_has_no_concrete_static_entries(&self, owner_name: &str) -> bool {
        if self
            .resolve_runtime_shadow_object_binding(owner_name)
            .is_some_and(|binding| {
                !self
                    .object_runtime_shadow_entries_from_binding(&binding)
                    .is_empty()
            })
        {
            return false;
        }

        self.runtime_object_property_shadow_copy_entries(owner_name)
            .into_iter()
            .all(|(_, fallback_value)| fallback_value.is_none())
    }

    fn runtime_shadow_object_binding_entries_match_ignoring_property(
        &self,
        left: &ObjectValueBinding,
        right: &ObjectValueBinding,
        ignored_property: Option<&Expression>,
    ) -> bool {
        let mut left_entries = self.object_runtime_shadow_entries_from_binding(left);
        let mut right_entries = self.object_runtime_shadow_entries_from_binding(right);
        if let Some(ignored_property) = ignored_property {
            let ignored_property =
                self.canonical_runtime_shadow_property_expression(ignored_property);
            left_entries.retain(|(property, _)| {
                !static_expression_matches(
                    &self.canonical_runtime_shadow_property_expression(property),
                    &ignored_property,
                )
            });
            right_entries.retain(|(property, _)| {
                !static_expression_matches(
                    &self.canonical_runtime_shadow_property_expression(property),
                    &ignored_property,
                )
            });
        }
        !left_entries.is_empty()
            && left_entries.len() == right_entries.len()
            && left_entries.iter().all(|(left_property, left_value)| {
                let left_property =
                    self.canonical_runtime_shadow_property_expression(left_property);
                right_entries.iter().any(|(right_property, right_value)| {
                    static_expression_matches(
                        &left_property,
                        &self.canonical_runtime_shadow_property_expression(right_property),
                    ) && static_expression_matches(left_value, right_value)
                })
            })
    }

    fn runtime_object_property_shadow_static_value_for_owner(
        &self,
        owner_name: &str,
        property: &Expression,
    ) -> Option<Expression> {
        let shadow_key = Self::runtime_object_property_shadow_key(property);
        let shadow_name = format!("__ayy_object_property__{owner_name}__{shadow_key}");
        let local_value = self
            .backend
            .global_semantics
            .values
            .value_bindings
            .get(&shadow_name)
            .cloned();
        let shared_value = self
            .backend
            .shared_global_semantics
            .values
            .value_bindings
            .get(&shadow_name)
            .cloned();
        match (local_value, shared_value) {
            (Some(local_value), Some(shared_value))
                if matches!(local_value, Expression::Null | Expression::Undefined)
                    && expression_may_evaluate_to_runtime_shadow_owner(&shared_value) =>
            {
                Some(shared_value)
            }
            (Some(local_value), Some(shared_value))
                if expression_may_evaluate_to_runtime_shadow_owner(&local_value)
                    && matches!(shared_value, Expression::Null | Expression::Undefined) =>
            {
                Some(local_value)
            }
            (Some(local_value), _) => Some(local_value),
            (None, shared_value) => shared_value,
        }
    }

    pub(in crate::backend::direct_wasm) fn runtime_object_property_shadow_value_is_statically_present_for_owner(
        &self,
        owner_name: &str,
        property: &Expression,
    ) -> bool {
        let property = self.canonical_runtime_shadow_property_expression(property);
        let shadow_key = Self::runtime_object_property_shadow_key(&property);
        let shadow_name = format!("__ayy_object_property__{owner_name}__{shadow_key}");
        if self
            .backend
            .global_property_descriptor(&shadow_name)
            .or_else(|| {
                self.backend
                    .shared_global_semantics
                    .values
                    .property_descriptor(&shadow_name)
            })
            .is_some_and(|descriptor| {
                descriptor.has_get
                    || descriptor.has_set
                    || descriptor.getter.is_some()
                    || descriptor.setter.is_some()
            })
        {
            return false;
        }
        if self
            .runtime_object_property_shadow_static_value_for_owner(owner_name, &property)
            .is_none()
        {
            return false;
        }

        let deleted_shadow_name =
            Self::runtime_object_property_deleted_shadow_name(owner_name, &property);
        let deleted_value_is_static = match self.global_value_binding(&deleted_shadow_name) {
            Some(Expression::Undefined) => true,
            Some(Expression::Number(number)) => *number == JS_UNDEFINED_TAG as f64,
            _ => false,
        };
        !(deleted_value_is_static
            || matches!(
                self.global_binding_kind(&deleted_shadow_name),
                Some(StaticValueKind::Undefined)
            ))
    }

    fn runtime_shadow_static_value_owner(&self, value: &Expression) -> Option<String> {
        if matches!(value, Expression::Call { .. })
            || self.expression_is_user_function_call_with_source_loop(value)
        {
            return None;
        }
        self.runtime_object_property_shadow_owner_name_for_expression(value)
            .or_else(|| {
                let materialized = self.materialize_static_expression(value);
                (!static_expression_matches(&materialized, value))
                    .then(|| {
                        self.runtime_object_property_shadow_owner_name_for_expression(&materialized)
                    })
                    .flatten()
            })
    }

    fn runtime_object_property_shadow_static_alias_owner_for_owner_property(
        &self,
        owner_name: &str,
        property: &Expression,
        member_owner: &str,
    ) -> Option<String> {
        self.runtime_object_property_shadow_static_value_for_owner(owner_name, property)
            .and_then(|value| self.runtime_shadow_static_value_owner(&value))
            .filter(|alias_owner| alias_owner != member_owner)
    }

    pub(in crate::backend::direct_wasm) fn runtime_object_property_shadow_binding_name_for_expression(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<String> {
        let owner_name = self.runtime_object_property_shadow_owner_name_for_expression(object)?;
        let property_name = static_property_name_from_expression(
            &self.canonical_object_property_expression(property),
        )?;
        Some(Self::runtime_object_property_shadow_binding_name(
            &owner_name,
            &property_name,
        ))
    }

    pub(in crate::backend::direct_wasm) fn runtime_object_property_shadow_binding_has_static_metadata(
        &self,
        shadow_binding_name: &str,
    ) -> bool {
        self.global_value_binding(shadow_binding_name).is_some()
            || self
                .backend
                .shared_global_semantics
                .values
                .value_bindings
                .contains_key(shadow_binding_name)
            || self.global_binding_kind(shadow_binding_name).is_some()
    }

    pub(in crate::backend::direct_wasm) fn runtime_object_property_shadow_binding_should_defer_static_resolution(
        &self,
        shadow_binding_name: &str,
    ) -> bool {
        self.global_has_implicit_binding(shadow_binding_name)
            && self.runtime_object_property_shadow_binding_has_static_metadata(shadow_binding_name)
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_accessor_runtime_object_property_shadow_value(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<Expression> {
        let property = self.canonical_object_property_expression(property);
        let shadow_binding_name =
            self.runtime_object_property_shadow_binding_name_for_expression(object, &property)?;
        let descriptor = self
            .backend
            .global_property_descriptor(&shadow_binding_name)?;
        let is_accessor = descriptor.has_get
            || descriptor.has_set
            || descriptor.getter.is_some()
            || descriptor.setter.is_some();
        if !is_accessor || descriptor.getter.is_none() {
            return None;
        }
        let value = self.global_value_binding(&shadow_binding_name)?;
        if matches!(value, Expression::Undefined)
            || matches!(value, Expression::Number(number) if *number == JS_UNDEFINED_TAG as f64)
        {
            return None;
        }
        Some(value.clone())
    }

    pub(in crate::backend::direct_wasm) fn runtime_object_property_shadow_deletion_may_hide_static_property(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> bool {
        let property = self.canonical_object_property_expression(property);
        let trace = crate::ayy_env_flag!("AYY_TRACE_RUNTIME_SHADOWS");
        let Some(owner_name) =
            self.runtime_object_property_shadow_owner_name_for_expression(object)
        else {
            if trace {
                eprintln!("shadow_may_hide object={object:?} property={property:?} owner=None");
            }
            return false;
        };
        let deleted_shadow_name =
            Self::runtime_object_property_deleted_shadow_name(&owner_name, &property);
        if !self.backend.delete_shadow_was_emitted(&deleted_shadow_name) {
            if trace {
                eprintln!(
                    "shadow_may_hide object={object:?} property={property:?} owner={owner_name} emitted=false name={deleted_shadow_name}"
                );
            }
            return false;
        }
        let object_binding = self
            .resolve_object_binding_from_expression(object)
            .or_else(|| match object {
                Expression::Identifier(name) => {
                    self.resolve_identifier_object_binding_fallback(name)
                }
                _ => None,
            });
        let result = !object_binding
            .as_ref()
            .is_some_and(|binding| object_binding_has_property(binding, &property));
        if trace {
            eprintln!(
                "shadow_may_hide object={object:?} property={property:?} owner={owner_name} emitted=true has_binding={} result={result} strings={:?} descriptors={:?}",
                object_binding.is_some(),
                object_binding.as_ref().map(|binding| binding
                    .string_properties
                    .iter()
                    .map(|(name, _)| name.clone())
                    .collect::<Vec<_>>()),
                object_binding.as_ref().map(|binding| binding
                    .property_descriptors
                    .iter()
                    .map(|(key, _)| key.clone())
                    .collect::<Vec<_>>()),
            );
        }
        result
    }

    pub(in crate::backend::direct_wasm) fn runtime_object_property_shadow_deletion_may_affect_property(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> bool {
        let property = self.canonical_object_property_expression(property);
        let Some(owner_name) =
            self.runtime_object_property_shadow_owner_name_for_expression(object)
        else {
            if crate::ayy_env_flag!("AYY_TRACE_RUNTIME_SHADOWS") {
                eprintln!("shadow_may_affect object={object:?} property={property:?} owner=None");
            }
            return false;
        };
        let deleted_shadow_name =
            Self::runtime_object_property_deleted_shadow_name(&owner_name, &property);
        let result = self.backend.delete_shadow_was_emitted(&deleted_shadow_name);
        if crate::ayy_env_flag!("AYY_TRACE_RUNTIME_SHADOWS") {
            eprintln!(
                "shadow_may_affect object={object:?} property={property:?} owner={owner_name} name={deleted_shadow_name} result={result}"
            );
        }
        result
    }

    pub(in crate::backend::direct_wasm) fn runtime_object_property_shadow_deletion_is_statically_present(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> bool {
        let property = self.canonical_object_property_expression(property);
        let Some(owner_name) =
            self.runtime_object_property_shadow_owner_name_for_expression(object)
        else {
            return false;
        };
        let deleted_shadow_name =
            Self::runtime_object_property_deleted_shadow_name(&owner_name, &property);
        let deleted_value_is_static = match self.global_value_binding(&deleted_shadow_name) {
            Some(Expression::Undefined) => true,
            Some(Expression::Number(number)) => *number == JS_UNDEFINED_TAG as f64,
            _ => false,
        };
        let result = deleted_value_is_static
            || matches!(
                self.global_binding_kind(&deleted_shadow_name),
                Some(StaticValueKind::Undefined)
            );
        if crate::ayy_env_flag!("AYY_TRACE_RUNTIME_SHADOWS") {
            eprintln!(
                "shadow_deletion_statically_present object={object:?} property={property:?} name={deleted_shadow_name} value={:?} kind={:?} result={result}",
                self.global_value_binding(&deleted_shadow_name),
                self.global_binding_kind(&deleted_shadow_name),
            );
        }
        result
    }

    pub(in crate::backend::direct_wasm) fn resolve_runtime_object_property_shadow_binding(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> Option<ImplicitGlobalBinding> {
        let property = self.canonical_object_property_expression(property);
        let owner_name = self.runtime_object_property_shadow_owner_name_for_expression(object)?;
        if crate::ayy_env_flag!("AYY_TRACE_RUNTIME_SHADOWS") {
            eprintln!(
                "runtime_shadow_binding object={object:?} property={property:?} owner={owner_name}"
            );
        }
        if let Expression::String(property_name) = property {
            return Some(self.ensure_implicit_global_binding(
                &Self::runtime_object_property_shadow_binding_name(&owner_name, &property_name),
            ));
        }
        let object_binding = self
            .resolve_object_binding_from_expression(object)
            .or_else(|| match object {
                Expression::Identifier(name) => {
                    self.resolve_identifier_object_binding_fallback(name)
                }
                _ => None,
            })?;
        object_binding_has_property(&object_binding, &property).then(|| {
            self.runtime_object_property_shadow_binding_by_property(&owner_name, &property)
        })
    }

    pub(in crate::backend::direct_wasm) fn resolve_runtime_object_property_shadow_deleted_binding(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> Option<ImplicitGlobalBinding> {
        let property = self.canonical_object_property_expression(property);
        let owner_name = self.runtime_object_property_shadow_owner_name_for_expression(object)?;
        if let Expression::String(property_name) = &property {
            return Some(self.ensure_implicit_global_binding(
                &Self::runtime_object_property_deleted_shadow_name(
                    &owner_name,
                    &Expression::String(property_name.clone()),
                ),
            ));
        }
        let object_binding = self
            .resolve_object_binding_from_expression(object)
            .or_else(|| match object {
                Expression::Identifier(name) => {
                    self.resolve_identifier_object_binding_fallback(name)
                }
                _ => None,
            })?;
        object_binding_has_property(&object_binding, &property).then(|| {
            self.runtime_object_property_shadow_deleted_binding_by_property(&owner_name, &property)
        })
    }

    pub(in crate::backend::direct_wasm) fn runtime_object_property_shadow_binding_by_property(
        &mut self,
        owner_name: &str,
        property: &Expression,
    ) -> ImplicitGlobalBinding {
        self.ensure_implicit_global_binding(&format!(
            "__ayy_object_property__{owner_name}__{}",
            Self::runtime_object_property_shadow_key(property)
        ))
    }

    pub(in crate::backend::direct_wasm) fn runtime_object_property_shadow_deleted_binding_by_property(
        &mut self,
        owner_name: &str,
        property: &Expression,
    ) -> ImplicitGlobalBinding {
        self.ensure_implicit_global_binding(&Self::runtime_object_property_deleted_shadow_name(
            owner_name, property,
        ))
    }

    pub(in crate::backend::direct_wasm) fn record_emitted_delete_shadow_for(
        &mut self,
        owner_name: &str,
        property: &Expression,
    ) {
        let name = Self::runtime_object_property_deleted_shadow_name(owner_name, property);
        self.backend.record_emitted_delete_shadow(&name);
    }

    pub(in crate::backend::direct_wasm) fn runtime_object_property_shadow_binding_by_names(
        &mut self,
        owner_name: &str,
        property_name: &str,
    ) -> ImplicitGlobalBinding {
        self.ensure_implicit_global_binding(&Self::runtime_object_property_shadow_binding_name(
            owner_name,
            property_name,
        ))
    }

    pub(in crate::backend::direct_wasm) fn runtime_object_dynamic_property_shadow_has_binding(
        &self,
        owner_name: &str,
    ) -> bool {
        self.global_has_implicit_binding(&Self::runtime_object_dynamic_property_key_shadow_name(
            owner_name,
        ))
    }

    pub(in crate::backend::direct_wasm) fn runtime_object_dynamic_property_key_shadow_binding(
        &mut self,
        owner_name: &str,
    ) -> ImplicitGlobalBinding {
        self.ensure_implicit_global_binding(&Self::runtime_object_dynamic_property_key_shadow_name(
            owner_name,
        ))
    }

    pub(in crate::backend::direct_wasm) fn runtime_object_dynamic_property_value_shadow_binding(
        &mut self,
        owner_name: &str,
    ) -> ImplicitGlobalBinding {
        self.ensure_implicit_global_binding(
            &Self::runtime_object_dynamic_property_value_shadow_name(owner_name),
        )
    }

    pub(in crate::backend::direct_wasm) fn emit_runtime_object_dynamic_property_shadow_store_from_locals(
        &mut self,
        owner_name: &str,
        property_local: u32,
        value_local: u32,
    ) {
        let key_binding = self.runtime_object_dynamic_property_key_shadow_binding(owner_name);
        let value_binding = self.runtime_object_dynamic_property_value_shadow_binding(owner_name);
        self.push_local_get(property_local);
        self.push_global_set(key_binding.value_index);
        self.push_i32_const(1);
        self.push_global_set(key_binding.present_index);
        self.push_local_get(value_local);
        self.push_global_set(value_binding.value_index);
        self.push_i32_const(1);
        self.push_global_set(value_binding.present_index);
    }

    pub(in crate::backend::direct_wasm) fn object_runtime_shadow_properties(
        &self,
        owner_name: &str,
    ) -> Vec<(Expression, Expression)> {
        let object_expression = Expression::Identifier(owner_name.to_string());
        let Some(object_binding) = self.resolve_object_binding_from_expression(&object_expression)
        else {
            return Vec::new();
        };
        let mut entries = self.object_runtime_shadow_entries_from_binding(&object_binding);
        self.filter_proxy_private_runtime_shadow_entries(owner_name, &mut entries);
        entries
    }

    fn collect_active_runtime_object_shadow_names_with_prefix(
        &self,
        prefix: &str,
    ) -> Vec<(u32, String)> {
        let mut seen = HashSet::new();
        let mut names = Vec::new();
        for name in self
            .backend
            .global_semantics
            .values
            .value_bindings
            .keys()
            .chain(
                self.backend
                    .global_semantics
                    .values
                    .property_descriptors
                    .keys(),
            )
            .chain(
                self.backend
                    .shared_global_semantics
                    .values
                    .value_bindings
                    .keys(),
            )
            .chain(
                self.backend
                    .shared_global_semantics
                    .values
                    .property_descriptors
                    .keys(),
            )
        {
            if !name.starts_with(prefix) || !seen.insert(name.clone()) {
                continue;
            }
            let Some(binding) = self
                .backend
                .global_semantics
                .global_names()
                .implicit_binding(name)
                .or_else(|| {
                    self.backend
                        .shared_global_semantics
                        .global_names()
                        .implicit_binding(name)
                })
            else {
                continue;
            };
            names.push((binding.value_index, name.clone()));
        }
        names.sort_by(|(left_index, left_name), (right_index, right_name)| {
            left_index
                .cmp(right_index)
                .then_with(|| left_name.cmp(right_name))
        });
        names
    }

    fn active_runtime_object_shadow_names_with_prefix(&self, prefix: &str) -> Vec<(u32, String)> {
        ACTIVE_RUNTIME_SHADOW_PREFIX_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.sync_generation();
            if let Some(names) = cache.names.get(prefix) {
                return names.clone();
            }
            let names = self.collect_active_runtime_object_shadow_names_with_prefix(prefix);
            cache.exists.insert(prefix.to_string(), !names.is_empty());
            cache.names.insert(prefix.to_string(), names.clone());
            names
        })
    }

    fn active_runtime_object_shadow_prefix_exists_uncached(&self, prefix: &str) -> bool {
        self.backend
            .global_semantics
            .values
            .value_bindings
            .keys()
            .chain(
                self.backend
                    .global_semantics
                    .values
                    .property_descriptors
                    .keys(),
            )
            .chain(
                self.backend
                    .shared_global_semantics
                    .values
                    .value_bindings
                    .keys(),
            )
            .chain(
                self.backend
                    .shared_global_semantics
                    .values
                    .property_descriptors
                    .keys(),
            )
            .any(|name| name.starts_with(prefix))
    }

    fn active_runtime_object_shadow_prefix_exists(&self, prefix: &str) -> bool {
        ACTIVE_RUNTIME_SHADOW_PREFIX_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.sync_generation();
            if let Some(exists) = cache.exists.get(prefix) {
                return *exists;
            }
            if let Some(names) = cache.names.get(prefix) {
                let exists = !names.is_empty();
                cache.exists.insert(prefix.to_string(), exists);
                return exists;
            }
            let exists = self.active_runtime_object_shadow_prefix_exists_uncached(prefix);
            cache.exists.insert(prefix.to_string(), exists);
            exists
        })
    }

    fn implicit_runtime_object_shadow_bindings_with_prefix(
        &self,
        prefix: &str,
    ) -> Vec<(String, ImplicitGlobalBinding)> {
        ACTIVE_RUNTIME_SHADOW_PREFIX_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.sync_generation();
            if let Some(bindings) = cache.implicit_bindings.get(prefix) {
                return bindings.clone();
            }
            let bindings = self
                .backend
                .global_semantics
                .global_names()
                .implicit_bindings
                .iter()
                .filter(|(name, _)| name.starts_with(prefix))
                .map(|(name, binding)| (name.clone(), *binding))
                .collect::<Vec<_>>();
            cache
                .implicit_bindings
                .insert(prefix.to_string(), bindings.clone());
            bindings
        })
    }

    pub(in crate::backend::direct_wasm) fn resolve_runtime_shadow_object_binding(
        &self,
        owner_name: &str,
    ) -> Option<ObjectValueBinding> {
        let prefix = format!("__ayy_object_property__{owner_name}__");
        let deleted_prefix = format!("__ayy_object_property_deleted__{owner_name}__");
        let resolved_owner_name = self
            .resolve_current_local_binding(owner_name)
            .map(|(resolved_name, _)| resolved_name)
            .unwrap_or_else(|| owner_name.to_string());
        let static_object_binding = self
            .state
            .speculation
            .static_semantics
            .local_object_binding(&resolved_owner_name)
            .or_else(|| {
                self.state
                    .speculation
                    .static_semantics
                    .local_object_binding(owner_name)
            })
            .cloned()
            .or_else(|| self.global_object_binding(owner_name).cloned())
            .or_else(|| self.global_prototype_object_binding(owner_name).cloned())
            .or_else(|| {
                self.resolve_user_function_capture_hidden_name(owner_name)
                    .and_then(|hidden_name| self.global_object_binding(&hidden_name).cloned())
            })
            .or_else(|| {
                self.resolve_object_binding_from_expression(&Expression::Identifier(
                    owner_name.to_string(),
                ))
            });
        let had_static_object_binding = static_object_binding.is_some();
        if crate::ayy_env_flag!("AYY_TRACE_RUNTIME_SHADOW_RESOLVE") {
            eprintln!(
                "runtime_shadow_resolve owner={owner_name} resolved_owner={resolved_owner_name} base={:?}",
                static_object_binding
                    .as_ref()
                    .map(object_binding_to_expression),
            );
        }
        let mut object_binding = static_object_binding.unwrap_or_else(empty_object_value_binding);
        self.filter_proxy_private_object_binding_entries(owner_name, &mut object_binding);
        let mut found_shadow_entry = false;
        let shadow_names = self.active_runtime_object_shadow_names_with_prefix(&prefix);
        let mut processed_shadow_names = Vec::new();
        for (_, name) in shadow_names {
            if processed_shadow_names.iter().any(|seen| seen == &name) {
                continue;
            }
            processed_shadow_names.push(name.clone());
            let Some(property_name) =
                Self::runtime_object_property_name_from_shadow_suffix(&name[prefix.len()..])
            else {
                continue;
            };
            let Some(value) = self.global_value_binding(&name).cloned().or_else(|| {
                self.backend
                    .shared_global_semantics
                    .values
                    .value_bindings
                    .get(&name)
                    .cloned()
            }) else {
                continue;
            };
            let property = Expression::String(property_name);
            if crate::ayy_env_flag!("AYY_TRACE_RUNTIME_SHADOW_RESOLVE") {
                eprintln!(
                    "runtime_shadow_resolve_override owner={owner_name} shadow={name} value={value:?}"
                );
            }
            if let Some(descriptor) = self.backend.global_property_descriptor(&name).or_else(|| {
                self.backend
                    .shared_global_semantics
                    .values
                    .property_descriptor(&name)
            }) {
                let is_accessor = descriptor.has_get
                    || descriptor.has_set
                    || descriptor.getter.is_some()
                    || descriptor.setter.is_some();
                object_binding_define_property_descriptor(
                    &mut object_binding,
                    property,
                    PropertyDescriptorBinding {
                        value: (!is_accessor).then(|| descriptor.value.clone()),
                        configurable: descriptor.configurable,
                        enumerable: descriptor.enumerable,
                        writable: descriptor.writable,
                        getter: descriptor.getter.clone(),
                        setter: descriptor.setter.clone(),
                        has_get: descriptor.has_get,
                        has_set: descriptor.has_set,
                    },
                );
            } else {
                object_binding_set_property(&mut object_binding, property, value);
            }
            found_shadow_entry = true;
        }
        let deleted_shadow_names =
            self.active_runtime_object_shadow_names_with_prefix(&deleted_prefix);
        let mut processed_deleted_shadow_names = Vec::new();
        for (_, name) in deleted_shadow_names {
            if processed_deleted_shadow_names
                .iter()
                .any(|seen| seen == &name)
            {
                continue;
            }
            processed_deleted_shadow_names.push(name.clone());
            let Some(property_name) = Self::runtime_object_property_name_from_shadow_suffix(
                &name[deleted_prefix.len()..],
            ) else {
                continue;
            };
            // A deletion marker is recorded as Undefined when the delete is
            // live and reset to Number(0.0) when a later store clears it; a
            // cleared marker must not hide the (re-added) property.
            let deleted_value = self.global_value_binding(&name).cloned().or_else(|| {
                self.backend
                    .shared_global_semantics
                    .values
                    .value_bindings
                    .get(&name)
                    .cloned()
            });
            let deleted_is_static = match deleted_value {
                Some(Expression::Undefined) => true,
                Some(Expression::Number(number)) => number == JS_UNDEFINED_TAG as f64,
                _ => false,
            };
            if deleted_is_static {
                object_binding_remove_property(
                    &mut object_binding,
                    &Expression::String(property_name),
                );
            }
        }

        (found_shadow_entry
            || had_static_object_binding
            || !object_binding.string_properties.is_empty()
            || !object_binding.symbol_properties.is_empty())
        .then_some(object_binding)
    }

    pub(in crate::backend::direct_wasm) fn emit_runtime_shadow_fallback_value(
        &mut self,
        fallback_value: &Expression,
    ) -> DirectResult<()> {
        if crate::ayy_env_flag!("AYY_TRACE_RUNTIME_SHADOWS") {
            eprintln!("runtime_shadow_fallback value={fallback_value:?}");
        }
        let Some(_fallback_guard) = RuntimeShadowFallbackGuard::enter(fallback_value) else {
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(());
        };
        if !self.runtime_shadow_fallback_references_readable_bindings(fallback_value) {
            self.push_i32_const(self.runtime_shadow_fallback_type_tag(fallback_value));
            return Ok(());
        }
        if !inline_summary_side_effect_free_expression(fallback_value) {
            self.push_i32_const(self.runtime_shadow_fallback_type_tag(fallback_value));
            return Ok(());
        }
        if let Expression::Identifier(name) = fallback_value
            && name.starts_with("__ayy_class_brand_")
        {
            if !self.emit_private_brand_runtime_value_for_binding_name(name)? {
                self.emit_private_brand_direct_or_synthetic_runtime_value_for_binding_name(name)?;
            }
            return Ok(());
        }
        if let Expression::Identifier(name) = fallback_value
            && name.starts_with("__ayy_closure_slot_")
            && let Some(private_brand_offset) = name.find("__ayy_class_brand_")
        {
            let private_brand_name = &name[private_brand_offset..];
            let emit_private_brand = |compiler: &mut Self| -> DirectResult<()> {
                if !compiler
                    .emit_private_brand_runtime_value_for_binding_name(private_brand_name)?
                {
                    compiler
                        .emit_private_brand_direct_or_synthetic_runtime_value_for_binding_name(
                            private_brand_name,
                        )?;
                }
                Ok(())
            };
            if let Some(hidden_binding) = self.hidden_implicit_global_binding(name) {
                self.push_global_get(hidden_binding.present_index);
                self.state.emission.output.instructions.push(0x04);
                self.state.emission.output.instructions.push(I32_TYPE);
                self.push_control_frame();
                self.push_global_get(hidden_binding.value_index);
                self.state.emission.output.instructions.push(0x05);
                emit_private_brand(self)?;
                self.state.emission.output.instructions.push(0x0b);
                self.pop_control_frame();
            } else {
                emit_private_brand(self)?;
            }
            return Ok(());
        }
        if let Some(function_binding) =
            self.resolve_function_binding_from_expression(fallback_value)
        {
            match function_binding {
                LocalFunctionBinding::User(function_name) => {
                    if let Some(user_function) = self.user_function(&function_name) {
                        self.push_i32_const(user_function_runtime_value(user_function));
                    } else {
                        self.push_i32_const(JS_UNDEFINED_TAG);
                    }
                }
                LocalFunctionBinding::Builtin(function_name) => {
                    self.push_i32_const(
                        builtin_function_runtime_value(&function_name)
                            .unwrap_or(JS_TYPEOF_FUNCTION_TAG),
                    );
                }
            }
            return Ok(());
        }

        if let Expression::Identifier(name) = fallback_value
            && (self.resolve_current_local_binding(name).is_some()
                || self.resolve_global_binding_index(name).is_some()
                || self.backend.implicit_global_binding(name).is_some()
                || self
                    .resolve_user_function_capture_hidden_name(name)
                    .is_some()
                || self.resolve_eval_local_function_hidden_name(name).is_some()
                || self.hidden_implicit_global_binding(name).is_some())
        {
            self.emit_numeric_expression(fallback_value)?;
            return Ok(());
        }

        if self
            .resolve_array_binding_from_expression(fallback_value)
            .is_some()
            || self
                .resolve_object_binding_from_expression(fallback_value)
                .is_some()
            || self
                .resolve_arguments_binding_from_expression(fallback_value)
                .is_some()
            || self
                .resolve_proxy_binding_from_expression(fallback_value)
                .is_some()
        {
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            return Ok(());
        }

        self.emit_numeric_expression(fallback_value)
    }

    fn runtime_shadow_fallback_type_tag(&self, fallback_value: &Expression) -> i32 {
        match self.infer_value_kind(fallback_value) {
            Some(StaticValueKind::Null) => JS_NULL_TAG,
            Some(StaticValueKind::Undefined) => JS_UNDEFINED_TAG,
            Some(kind) => kind.as_typeof_tag().unwrap_or(JS_UNDEFINED_TAG),
            None => JS_UNDEFINED_TAG,
        }
    }

    fn runtime_shadow_fallback_identifier_is_readable(&self, name: &str) -> bool {
        self.parameter_scope_arguments_local_for(name).is_some()
            || (self.is_current_arguments_binding_name(name) && self.has_arguments_object())
            || self.resolve_current_local_binding(name).is_some()
            || self
                .state
                .speculation
                .static_semantics
                .has_local_function_binding(name)
            || self.resolve_global_binding_index(name).is_some()
            || self.backend.implicit_global_binding(name).is_some()
            || self
                .resolve_user_function_capture_hidden_name(name)
                .is_some()
            || self.resolve_eval_local_function_hidden_name(name).is_some()
            || parse_test262_realm_identifier(name).is_some()
            || parse_test262_realm_global_identifier(name).is_some()
            || (name == "NaN" && self.is_unshadowed_builtin_identifier(name))
            || (name == "Infinity" && self.is_unshadowed_builtin_identifier(name))
            || name == "undefined"
            || builtin_function_runtime_value(name).is_some()
            || (is_internal_user_function_identifier(name)
                && self.user_function_runtime_value(name).is_some())
            || self.lookup_identifier_kind(name).is_some()
            || name.find("__ayy_class_brand_").is_some()
            || (name.starts_with("__ayy_class_super_")
                && self
                    .resolve_static_class_init_local_alias_expression(name)
                    .filter(|resolved| {
                        !static_expression_matches(
                            resolved,
                            &Expression::Identifier(name.to_string()),
                        )
                    })
                    .is_some())
            || name == "__ayy_null_super_constructor"
    }

    pub(in crate::backend::direct_wasm) fn runtime_shadow_fallback_references_readable_bindings(
        &self,
        fallback_value: &Expression,
    ) -> bool {
        let mut referenced_names = HashSet::new();
        collect_referenced_binding_names_from_expression(fallback_value, &mut referenced_names);
        referenced_names
            .iter()
            .all(|name| self.runtime_shadow_fallback_identifier_is_readable(name))
    }

    pub(in crate::backend::direct_wasm) fn sync_runtime_object_property_shadow_static_metadata_from_binding(
        &mut self,
        target_owner: &str,
        object_binding: &ObjectValueBinding,
    ) {
        let trace_shadow_timing = crate::ayy_env_flag!("AYY_TRACE_RUNTIME_SHADOW_TIMING");
        let total_start = trace_shadow_timing.then(std::time::Instant::now);
        for (property, fallback_value) in
            self.object_runtime_shadow_entries_from_binding(object_binding)
        {
            let property_start = trace_shadow_timing.then(std::time::Instant::now);
            let descriptor = object_binding_lookup_descriptor(object_binding, &property);
            let getter_this_binding = if target_owner == "this" {
                Expression::This
            } else {
                Expression::Identifier(target_owner.to_string())
            };
            let getter_return_value =
                if descriptor.is_some_and(Self::property_descriptor_is_accessor) {
                    descriptor
                        .and_then(|descriptor| descriptor.getter.as_ref())
                        .and_then(|getter| self.resolve_function_binding_from_expression(getter))
                        .and_then(|getter_binding| {
                            self.resolve_static_getter_value_from_binding_with_context(
                                &getter_binding,
                                &getter_this_binding,
                                self.current_function_name(),
                            )
                        })
                } else {
                    None
                };
            let has_getter_return_value = getter_return_value.is_some();
            let mut fallback_value = getter_return_value
                .as_ref()
                .cloned()
                .unwrap_or(fallback_value);
            fallback_value =
                self.rewrite_static_new_this_expression_for_owner(&fallback_value, target_owner);
            let member_owner =
                Self::runtime_object_member_shadow_owner_name(target_owner, &property);
            if !has_getter_return_value {
                let current_this_member_owner =
                    Self::runtime_object_member_shadow_owner_name("this", &property);
                if target_owner != "this"
                    && matches!(
                        &fallback_value,
                        Expression::Identifier(name) if name == &current_this_member_owner
                    )
                {
                    fallback_value = Expression::Identifier(target_owner.to_string());
                } else if self.runtime_shadow_value_may_have_member_shadows(&fallback_value)
                    && self.runtime_object_property_shadow_owner_has_bindings(&member_owner)
                {
                    fallback_value = Expression::Identifier(member_owner);
                }
            }
            if Self::runtime_shadow_class_entry_should_defer(target_owner, &fallback_value) {
                if crate::ayy_env_flag!("AYY_TRACE_RUNTIME_SHADOWS") {
                    eprintln!(
                        "runtime_shadow_static_sync_defer_class target={target_owner} property={property:?} fallback={fallback_value:?}"
                    );
                }
                continue;
            }
            let shadow_binding_name = format!(
                "__ayy_object_property__{target_owner}__{}",
                Self::runtime_object_property_shadow_key(&property)
            );
            if !has_getter_return_value
                && descriptor.is_some_and(Self::property_descriptor_is_accessor)
            {
                self.backend
                    .clear_global_static_binding_metadata(&shadow_binding_name);
                self.backend
                    .clear_shared_global_static_binding_metadata(&shadow_binding_name);
                if crate::ayy_env_flag!("AYY_TRACE_RUNTIME_SHADOWS") {
                    eprintln!(
                        "runtime_shadow_static_sync_defer_accessor target={target_owner} property={property:?}"
                    );
                }
                continue;
            }
            self.ensure_implicit_global_binding(&shadow_binding_name);
            if Self::expression_is_runtime_object_property_shadow_identifier(&fallback_value) {
                if crate::ayy_env_flag!("AYY_TRACE_RUNTIME_SHADOWS") {
                    eprintln!(
                        "runtime_shadow_static_sync_skip_shadow_identifier target={target_owner} property={property:?} fallback={fallback_value:?}"
                    );
                }
                continue;
            }
            // The deleted marker is created on demand by delete emission;
            // ensuring it here made every synced property look deletable,
            // poisoning static descriptor and kind resolution.
            let metadata_value = self
                .resolve_runtime_shadow_static_sync_current_binding_primitive(
                    &fallback_value,
                    target_owner,
                    object_binding,
                )
                .or_else(|| match &fallback_value {
                    Expression::Call { callee, arguments } => self
                        .resolve_effectful_call_return_metadata_value(callee, arguments)
                        .or_else(|| {
                            self.resolve_static_call_result_expression_with_context(
                                callee,
                                arguments,
                                self.current_function_name(),
                            )
                            .map(|(value, _)| value)
                        }),
                    _ => None,
                })
                .or_else(|| {
                    self.resolve_static_primitive_expression_with_context(
                        &fallback_value,
                        self.current_function_name(),
                    )
                })
                .unwrap_or_else(|| fallback_value.clone());
            let materialized_value =
                self.reference_preserving_static_value_expression(&metadata_value);
            if crate::ayy_env_flag!("AYY_TRACE_RUNTIME_SHADOWS") {
                eprintln!(
                    "runtime_shadow_static_sync target={target_owner} property={property:?} fallback={fallback_value:?} materialized={materialized_value:?}"
                );
            }
            self.update_static_global_assignment_metadata(
                &shadow_binding_name,
                &materialized_value,
            );
            if let Some(binding_property) = self.member_function_binding_property(&property) {
                let key = MemberFunctionBindingKey {
                    target: MemberFunctionBindingTarget::Identifier(target_owner.to_string()),
                    property: binding_property,
                };
                if let Some(binding) =
                    self.resolve_function_binding_from_expression(&materialized_value)
                {
                    self.backend
                        .set_global_member_function_binding(key.clone(), binding);
                } else {
                    self.backend.clear_global_member_function_binding(&key);
                }
                if let Some(descriptor) = descriptor {
                    if let Some(binding) = descriptor
                        .getter
                        .as_ref()
                        .and_then(|getter| self.resolve_function_binding_from_expression(getter))
                    {
                        self.backend
                            .set_global_member_getter_binding(key.clone(), binding);
                    } else if descriptor.has_get || descriptor.getter.is_some() {
                        self.backend.clear_global_member_getter_binding(&key);
                    }
                    if let Some(binding) = descriptor
                        .setter
                        .as_ref()
                        .and_then(|setter| self.resolve_function_binding_from_expression(setter))
                    {
                        self.backend
                            .set_global_member_setter_binding(key.clone(), binding);
                    } else if descriptor.has_set || descriptor.setter.is_some() {
                        self.backend.clear_global_member_setter_binding(&key);
                    }
                }
            }
            if let Some(descriptor) = descriptor {
                let descriptor_state = GlobalPropertyDescriptorState {
                    value: descriptor
                        .value
                        .as_ref()
                        .map(|value| self.materialize_static_expression(value))
                        .or_else(|| getter_return_value.clone())
                        .unwrap_or(Expression::Undefined),
                    writable: descriptor.writable,
                    enumerable: descriptor.enumerable,
                    configurable: descriptor.configurable,
                    getter: descriptor.getter.clone(),
                    setter: descriptor.setter.clone(),
                    has_get: descriptor.has_get,
                    has_set: descriptor.has_set,
                };
                self.backend.upsert_global_property_descriptor(
                    shadow_binding_name.clone(),
                    descriptor_state.clone(),
                );
                crate::backend::direct_wasm::memo::bump_static_state_generation();
                self.backend
                    .shared_global_semantics
                    .values
                    .property_descriptors
                    .insert(shadow_binding_name.clone(), descriptor_state);
            } else if let Expression::String(property_name) = &property
                && object_binding
                    .non_enumerable_string_properties
                    .iter()
                    .any(|hidden_name| hidden_name == property_name)
            {
                let descriptor_state = GlobalPropertyDescriptorState {
                    value: materialized_value.clone(),
                    writable: Some(true),
                    enumerable: false,
                    configurable: true,
                    getter: None,
                    setter: None,
                    has_get: false,
                    has_set: false,
                };
                self.backend.upsert_global_property_descriptor(
                    shadow_binding_name.clone(),
                    descriptor_state.clone(),
                );
                crate::backend::direct_wasm::memo::bump_static_state_generation();
                self.backend
                    .shared_global_semantics
                    .values
                    .property_descriptors
                    .insert(shadow_binding_name.clone(), descriptor_state);
            } else if let Some(stale_descriptor) = self
                .backend
                .global_property_descriptor(&shadow_binding_name)
                .cloned()
                .or_else(|| {
                    self.backend
                        .shared_global_semantics
                        .values
                        .property_descriptor(&shadow_binding_name)
                        .cloned()
                })
                .filter(|state| {
                    !state.has_get
                        && !state.has_set
                        && state.getter.is_none()
                        && state.setter.is_none()
                })
                .filter(|state| !static_expression_matches(&state.value, &materialized_value))
            {
                // The synced binding carries no descriptor for this property,
                // but a previously copied/seeded shadow descriptor still holds
                // the pre-call value. Refresh the data value so static
                // resolution does not fall back to the stale snapshot.
                let descriptor_state = GlobalPropertyDescriptorState {
                    value: materialized_value.clone(),
                    ..stale_descriptor
                };
                self.backend.upsert_global_property_descriptor(
                    shadow_binding_name.clone(),
                    descriptor_state.clone(),
                );
                crate::backend::direct_wasm::memo::bump_static_state_generation();
                self.backend
                    .shared_global_semantics
                    .values
                    .property_descriptors
                    .insert(shadow_binding_name.clone(), descriptor_state);
            }
            self.backend
                .shared_global_semantics
                .values
                .set_value_binding(shadow_binding_name.clone(), materialized_value.clone());
            if let Some(kind) = self.infer_value_kind(&materialized_value) {
                self.backend
                    .shared_global_semantics
                    .set_global_binding_kind(&shadow_binding_name, kind);
            }
            if let Some(property_start) = property_start {
                eprintln!(
                    "runtime_shadow_static_sync_timing target={target_owner} property={property:?} elapsed_ms={}",
                    property_start.elapsed().as_millis()
                );
            }
        }
        if let Some(total_start) = total_start {
            eprintln!(
                "runtime_shadow_static_sync_total target={target_owner} elapsed_ms={}",
                total_start.elapsed().as_millis()
            );
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_effectful_call_return_metadata_value(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Option<Expression> {
        let LocalFunctionBinding::User(function_name) =
            self.resolve_function_binding_from_expression(callee)?
        else {
            return None;
        };
        let user_function = self.user_function(&function_name)?;
        if user_function.has_lowered_pattern_parameters() || user_function.has_parameter_defaults()
        {
            return None;
        }
        let return_value = user_function
            .inline_summary
            .as_ref()?
            .return_value
            .as_ref()?;
        let expanded_arguments = self.expand_call_arguments(arguments);
        let arguments_binding = Expression::Array(
            expanded_arguments
                .iter()
                .cloned()
                .map(ArrayElement::Expression)
                .collect(),
        );
        Some(self.substitute_user_function_call_frame_bindings(
            return_value,
            user_function,
            arguments,
            &Expression::Undefined,
            &arguments_binding,
        ))
    }

    pub(in crate::backend::direct_wasm) fn emit_runtime_object_property_shadow_seed_from_binding(
        &mut self,
        target_owner: &str,
        object_binding: &ObjectValueBinding,
    ) -> DirectResult<()> {
        let target_expression = Expression::Identifier(target_owner.to_string());
        self.emit_runtime_object_property_shadow_seed_from_binding_with_receiver(
            target_owner,
            object_binding,
            &target_expression,
        )
    }

    pub(in crate::backend::direct_wasm) fn emit_runtime_object_property_shadow_seed_from_binding_with_receiver(
        &mut self,
        target_owner: &str,
        object_binding: &ObjectValueBinding,
        _getter_this_expression: &Expression,
    ) -> DirectResult<()> {
        let target_expression = Expression::Identifier(target_owner.to_string());
        let mut properties = ordered_object_property_names(object_binding)
            .into_iter()
            .map(|property_name| {
                let property = Expression::String(property_name);
                (property.clone(), property)
            })
            .collect::<Vec<_>>();
        properties.extend(
            object_binding
                .symbol_properties
                .iter()
                .map(|(property, _)| {
                    (
                        property.clone(),
                        self.canonical_runtime_shadow_property_expression(property),
                    )
                }),
        );
        for (raw_property, property) in properties {
            if Self::runtime_shadow_owner_is_class_object(target_owner)
                && Self::runtime_shadow_property_is_private(&property)
            {
                let is_static_private_marker = matches!(
                    &property,
                    Expression::String(property_name)
                        if property_name.starts_with("__ayy$private_brand$")
                ) && matches!(
                    object_binding_lookup_value(object_binding, &property),
                    Some(Expression::Bool(true))
                ) && self
                    .class_init_defines_static_private_marker(target_owner, &property);
                if !is_static_private_marker {
                    continue;
                }
            }
            let descriptor = object_binding_lookup_descriptor(object_binding, &property)
                .or_else(|| object_binding_lookup_descriptor(object_binding, &raw_property));
            let fallback_value = object_binding_lookup_value(object_binding, &property)
                .or_else(|| object_binding_lookup_value(object_binding, &raw_property))
                .cloned()
                .unwrap_or(Expression::Undefined);
            let fallback_value =
                self.rewrite_static_new_this_expression_for_owner(&fallback_value, target_owner);
            if Self::runtime_shadow_class_entry_should_defer(target_owner, &fallback_value) {
                if crate::ayy_env_flag!("AYY_TRACE_RUNTIME_SHADOWS") {
                    eprintln!(
                        "runtime_shadow_seed_defer_class target={target_owner} property={property:?} fallback={fallback_value:?}"
                    );
                }
                continue;
            }
            let target_binding =
                self.runtime_object_property_shadow_binding_by_property(target_owner, &property);
            let target_deleted = self.runtime_object_property_shadow_deleted_binding_by_property(
                target_owner,
                &property,
            );
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(target_deleted.value_index);
            self.push_i32_const(0);
            self.push_global_set(target_deleted.present_index);
            let seed_accessor_placeholder = descriptor.is_some_and(|descriptor| {
                descriptor.has_get
                    || descriptor.has_set
                    || descriptor.getter.is_some()
                    || descriptor.setter.is_some()
            });
            if seed_accessor_placeholder {
                // Accessor properties must not execute while seeding shadow slots. The
                // member-read shadow fallback resolves descriptor getters when no
                // runtime data shadow is present, so the slot stays absent until a
                // write actually creates a shadow value.
                self.push_i32_const(JS_UNDEFINED_TAG);
            } else if !self.emit_private_brand_marker_runtime_value(
                &target_expression,
                &property,
                &fallback_value,
            )? {
                let seed_value = self
                    .resolve_runtime_shadow_static_sync_current_binding_primitive(
                        &fallback_value,
                        target_owner,
                        object_binding,
                    )
                    .or_else(|| self.runtime_shadow_static_sync_seed_value(&fallback_value))
                    .unwrap_or(fallback_value);
                self.emit_runtime_shadow_fallback_value(&seed_value)?;
            }
            self.push_global_set(target_binding.value_index);
            self.push_i32_const(if seed_accessor_placeholder { 0 } else { 1 });
            self.push_global_set(target_binding.present_index);
        }
        Ok(())
    }

    fn emit_runtime_object_property_shadow_prefix_copy(
        &mut self,
        source_owner: &str,
        target_owner: &str,
    ) -> DirectResult<()> {
        let handled_suffixes = self
            .runtime_object_property_shadow_copy_entries(source_owner)
            .into_iter()
            .map(|(property, _)| Self::runtime_object_property_shadow_key(&property))
            .collect::<HashSet<_>>();
        let source_prefix = format!("__ayy_object_property__{source_owner}__");
        let source_deleted_prefix = format!("__ayy_object_property_deleted__{source_owner}__");
        let implicit_bindings = self
            .backend
            .global_semantics
            .global_names()
            .implicit_bindings
            .iter()
            .map(|(name, binding)| (name.clone(), *binding))
            .collect::<Vec<_>>();
        let mut suffix_bindings: BTreeMap<
            String,
            (Option<ImplicitGlobalBinding>, Option<ImplicitGlobalBinding>),
        > = BTreeMap::new();

        for (name, binding) in implicit_bindings {
            if let Some(suffix) = name.strip_prefix(&source_prefix) {
                if handled_suffixes.contains(suffix) {
                    continue;
                }
                suffix_bindings.entry(suffix.to_string()).or_default().0 = Some(binding);
                continue;
            }

            let Some(suffix) = name.strip_prefix(&source_deleted_prefix) else {
                continue;
            };
            if handled_suffixes.contains(suffix) {
                continue;
            }
            suffix_bindings.entry(suffix.to_string()).or_default().1 = Some(binding);
        }

        for (suffix, (source_binding, source_deleted)) in suffix_bindings {
            let private_shadow_property_name =
                Self::runtime_object_property_name_from_shadow_suffix(&suffix).filter(
                    |property_name| {
                        property_name.starts_with("__ayy$private$")
                            || property_name.starts_with("__ayy$private_brand$")
                    },
                );
            if crate::ayy_env_flag!("AYY_TRACE_RUNTIME_SHADOWS") {
                eprintln!(
                    "runtime_shadow_prefix_copy {source_owner}->{target_owner} suffix={suffix} source_binding={} source_deleted={}",
                    source_binding.is_some(),
                    source_deleted.is_some()
                );
            }
            let target_binding = self.ensure_implicit_global_binding(&format!(
                "__ayy_object_property__{target_owner}__{suffix}"
            ));
            let target_deleted = self.ensure_implicit_global_binding(&format!(
                "__ayy_object_property_deleted__{target_owner}__{suffix}"
            ));

            if let Some(source_deleted) = source_deleted {
                self.push_global_get(source_deleted.present_index);
                self.state.emission.output.instructions.push(0x04);
                self.state
                    .emission
                    .output
                    .instructions
                    .push(EMPTY_BLOCK_TYPE);
                self.push_control_frame();
                self.push_i32_const(JS_UNDEFINED_TAG);
                self.push_global_set(target_binding.value_index);
                self.push_i32_const(0);
                self.push_global_set(target_binding.present_index);
                self.push_i32_const(JS_UNDEFINED_TAG);
                self.push_global_set(target_deleted.value_index);
                self.push_i32_const(1);
                self.push_global_set(target_deleted.present_index);
                self.state.emission.output.instructions.push(0x05);
                if let Some(source_binding) = source_binding {
                    self.push_global_get(source_binding.present_index);
                    self.state.emission.output.instructions.push(0x04);
                    self.state
                        .emission
                        .output
                        .instructions
                        .push(EMPTY_BLOCK_TYPE);
                    self.push_control_frame();
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    self.push_global_set(target_deleted.value_index);
                    self.push_i32_const(0);
                    self.push_global_set(target_deleted.present_index);
                    if crate::ayy_env_flag!("AYY_TRACE_PRIVATE_MEMBER_VALUES")
                        && private_shadow_property_name.is_some()
                    {
                        let copied_value_local = self.allocate_temp_local();
                        self.push_global_get(source_binding.value_index);
                        self.push_local_set(copied_value_local);
                        self.emit_runtime_shadow_debug_print_local(
                            &format!(
                                "private_shadow_prefix_copy {source_owner}->{target_owner} {}",
                                private_shadow_property_name
                                    .as_deref()
                                    .unwrap_or(suffix.as_str())
                            ),
                            copied_value_local,
                        )?;
                        self.push_local_get(copied_value_local);
                    } else {
                        self.push_global_get(source_binding.value_index);
                    }
                    self.push_global_set(target_binding.value_index);
                    self.push_i32_const(1);
                    self.push_global_set(target_binding.present_index);
                    self.state.emission.output.instructions.push(0x05);
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    self.push_global_set(target_binding.value_index);
                    self.push_i32_const(0);
                    self.push_global_set(target_binding.present_index);
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    self.push_global_set(target_deleted.value_index);
                    self.push_i32_const(0);
                    self.push_global_set(target_deleted.present_index);
                    self.state.emission.output.instructions.push(0x0b);
                    self.pop_control_frame();
                } else {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    self.push_global_set(target_binding.value_index);
                    self.push_i32_const(0);
                    self.push_global_set(target_binding.present_index);
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    self.push_global_set(target_deleted.value_index);
                    self.push_i32_const(0);
                    self.push_global_set(target_deleted.present_index);
                }
                self.state.emission.output.instructions.push(0x0b);
                self.pop_control_frame();
                continue;
            }

            if let Some(source_binding) = source_binding {
                if crate::ayy_env_flag!("AYY_TRACE_PRIVATE_MEMBER_VALUES")
                    && private_shadow_property_name.is_some()
                {
                    let copied_value_local = self.allocate_temp_local();
                    self.push_global_get(source_binding.value_index);
                    self.push_local_set(copied_value_local);
                    self.emit_runtime_shadow_debug_print_local(
                        &format!(
                            "private_shadow_prefix_copy {source_owner}->{target_owner} {}",
                            private_shadow_property_name
                                .as_deref()
                                .unwrap_or(suffix.as_str())
                        ),
                        copied_value_local,
                    )?;
                    self.push_local_get(copied_value_local);
                } else {
                    self.push_global_get(source_binding.value_index);
                }
                self.push_global_set(target_binding.value_index);
                self.push_global_get(source_binding.present_index);
                self.push_global_set(target_binding.present_index);
                self.push_i32_const(JS_UNDEFINED_TAG);
                self.push_global_set(target_deleted.value_index);
                self.push_i32_const(0);
                self.push_global_set(target_deleted.present_index);
                continue;
            }

            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(target_binding.value_index);
            self.push_i32_const(0);
            self.push_global_set(target_binding.present_index);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(target_deleted.value_index);
            self.push_i32_const(0);
            self.push_global_set(target_deleted.present_index);
        }
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn clear_runtime_object_property_shadow_prefix(
        &mut self,
        owner_name: &str,
    ) {
        if crate::ayy_env_flag!("AYY_TRACE_RUNTIME_SHADOWS") {
            eprintln!("runtime_shadow_clear_prefix {owner_name}");
        }
        let property_prefix = format!("__ayy_object_property__{owner_name}__");
        let deleted_prefix = format!("__ayy_object_property_deleted__{owner_name}__");
        let mut implicit_bindings =
            self.implicit_runtime_object_shadow_bindings_with_prefix(&property_prefix);
        implicit_bindings
            .extend(self.implicit_runtime_object_shadow_bindings_with_prefix(&deleted_prefix));

        for (_, binding) in implicit_bindings {
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(binding.value_index);
            self.push_i32_const(0);
            self.push_global_set(binding.present_index);
        }
    }

    pub(in crate::backend::direct_wasm) fn clear_runtime_object_property_non_private_shadow_prefix(
        &mut self,
        owner_name: &str,
    ) {
        if crate::ayy_env_flag!("AYY_TRACE_RUNTIME_SHADOWS") {
            eprintln!("runtime_shadow_clear_non_private_prefix {owner_name}");
        }
        let property_prefix = format!("__ayy_object_property__{owner_name}__");
        let deleted_prefix = format!("__ayy_object_property_deleted__{owner_name}__");
        let mut implicit_bindings =
            self.implicit_runtime_object_shadow_bindings_with_prefix(&property_prefix);
        implicit_bindings
            .extend(self.implicit_runtime_object_shadow_bindings_with_prefix(&deleted_prefix));

        for (name, binding) in implicit_bindings {
            let suffix = name
                .strip_prefix(&property_prefix)
                .or_else(|| name.strip_prefix(&deleted_prefix));
            let Some(suffix) = suffix else {
                continue;
            };
            if Self::runtime_object_property_name_from_shadow_suffix(suffix).is_some_and(
                |property_name| {
                    property_name.starts_with("__ayy$private$")
                        || property_name.starts_with("__ayy$private_brand$")
                },
            ) {
                continue;
            }
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(binding.value_index);
            self.push_i32_const(0);
            self.push_global_set(binding.present_index);
        }
    }

    pub(in crate::backend::direct_wasm) fn clear_runtime_object_property_shadow_static_metadata_prefix(
        &mut self,
        owner_name: &str,
    ) {
        let property_prefix = format!("__ayy_object_property__{owner_name}__");
        let deleted_prefix = format!("__ayy_object_property_deleted__{owner_name}__");
        let names = self
            .backend
            .global_semantics
            .global_names()
            .implicit_bindings
            .keys()
            .chain(
                self.backend
                    .shared_global_semantics
                    .global_names()
                    .implicit_bindings
                    .keys(),
            )
            .filter(|name| name.starts_with(&property_prefix) || name.starts_with(&deleted_prefix))
            .cloned()
            .collect::<HashSet<_>>();

        for name in names {
            self.backend
                .global_semantics
                .clear_global_binding_state(&name);
            self.backend
                .shared_global_semantics
                .clear_global_binding_state(&name);
        }
    }

    pub(in crate::backend::direct_wasm) fn sync_runtime_object_shadow_owner_static_metadata_from_expression(
        &mut self,
        owner_name: &str,
        updated_expression: &Expression,
    ) {
        let updated_expression = self.materialize_static_expression(updated_expression);
        let Some(updated_object_binding) =
            self.resolve_object_binding_from_expression(&updated_expression)
        else {
            return;
        };

        self.clear_runtime_object_property_shadow_static_metadata_prefix(owner_name);
        self.sync_runtime_object_property_shadow_static_metadata_from_binding(
            owner_name,
            &updated_object_binding,
        );

        let resolved_identifier_name = self
            .resolve_current_local_binding(owner_name)
            .map(|(resolved_name, _)| resolved_name)
            .filter(|resolved_name| resolved_name != owner_name);
        if let Some(resolved_name) = resolved_identifier_name.as_deref() {
            self.update_local_object_binding(resolved_name, &updated_expression);
        }
        self.update_local_object_binding(owner_name, &updated_expression);
    }

    pub(in crate::backend::direct_wasm) fn runtime_shadow_owner_should_preserve_function_identity(
        &self,
        owner_name: &str,
    ) -> bool {
        owner_name.starts_with("__ayy_class_expr_")
            || owner_name.starts_with("__ayy_class_ctor_")
            || self
                .resolve_function_binding_from_expression(&Expression::Identifier(
                    owner_name.to_string(),
                ))
                .is_some()
            || self
                .state
                .speculation
                .static_semantics
                .local_function_binding(owner_name)
                .is_some()
            || self.backend.global_function_binding(owner_name).is_some()
    }

    pub(in crate::backend::direct_wasm) fn sync_runtime_object_shadow_owner_static_metadata_from_binding(
        &mut self,
        owner_name: &str,
        updated_object_binding: &ObjectValueBinding,
    ) {
        let updated_expression = object_binding_to_expression(updated_object_binding);
        self.clear_runtime_object_property_shadow_static_metadata_prefix(owner_name);
        self.sync_runtime_object_property_shadow_static_metadata_from_binding(
            owner_name,
            updated_object_binding,
        );

        let resolved_identifier_name = self
            .resolve_current_local_binding(owner_name)
            .map(|(resolved_name, _)| resolved_name)
            .filter(|resolved_name| resolved_name != owner_name);
        if let Some(resolved_name) = resolved_identifier_name.as_deref() {
            let preserve_function_identity =
                self.runtime_shadow_owner_should_preserve_function_identity(resolved_name);
            self.state
                .speculation
                .static_semantics
                .set_local_object_binding(resolved_name, updated_object_binding.clone());
            if preserve_function_identity {
                self.state
                    .speculation
                    .static_semantics
                    .set_local_kind(resolved_name, StaticValueKind::Function);
            } else {
                self.update_local_value_binding(resolved_name, &updated_expression);
                self.state
                    .speculation
                    .static_semantics
                    .set_local_kind(resolved_name, StaticValueKind::Object);
            }
        }
        let preserve_function_identity =
            self.runtime_shadow_owner_should_preserve_function_identity(owner_name);
        self.state
            .speculation
            .static_semantics
            .set_local_object_binding(owner_name, updated_object_binding.clone());
        if preserve_function_identity {
            self.state
                .speculation
                .static_semantics
                .set_local_kind(owner_name, StaticValueKind::Function);
        } else {
            self.update_local_value_binding(owner_name, &updated_expression);
            self.state
                .speculation
                .static_semantics
                .set_local_kind(owner_name, StaticValueKind::Object);
        }
        if self.binding_name_is_global(owner_name)
            || self.backend.global_has_binding(owner_name)
            || self.backend.global_has_lexical_binding(owner_name)
            || self.global_has_implicit_binding(owner_name)
        {
            if preserve_function_identity {
                self.backend
                    .sync_global_object_binding(owner_name, Some(updated_object_binding.clone()));
                self.backend
                    .set_global_binding_kind(owner_name, StaticValueKind::Function);
            } else {
                self.update_static_global_assignment_metadata(owner_name, &updated_expression);
            }
        }
    }

    pub(in crate::backend::direct_wasm) fn sync_user_function_parameter_object_shadow_writeback_static_metadata(
        &mut self,
        writebacks: &[(String, String, Option<ObjectValueBinding>)],
        updated_bindings: Option<&HashMap<String, Expression>>,
    ) {
        let Some(updated_bindings) = updated_bindings else {
            for (param_name, source_owner, source_object_binding) in writebacks {
                let Some(updated_object_binding) = self
                    .resolve_runtime_shadow_object_binding(param_name)
                    .or_else(|| source_object_binding.as_ref().cloned())
                else {
                    continue;
                };
                self.sync_runtime_object_shadow_owner_static_metadata_from_binding(
                    param_name,
                    &updated_object_binding,
                );
                self.sync_runtime_object_shadow_owner_static_metadata_from_binding(
                    source_owner,
                    &updated_object_binding,
                );
            }
            return;
        };

        for (param_name, source_owner, _) in writebacks {
            if crate::ayy_env_flag!("AYY_TRACE_RUNTIME_SHADOWS") {
                eprintln!(
                    "runtime_shadow_param_writeback_sync param={param_name} source_owner={source_owner} updated_binding={:?}",
                    updated_bindings.get(param_name),
                    param_name = param_name,
                    source_owner = source_owner,
                );
            }
            let Some(updated_expression) = updated_bindings.get(param_name) else {
                let Some(updated_expression) = updated_bindings.get(source_owner) else {
                    continue;
                };
                self.sync_runtime_object_shadow_owner_static_metadata_from_expression(
                    param_name,
                    updated_expression,
                );
                self.sync_runtime_object_shadow_owner_static_metadata_from_expression(
                    source_owner,
                    updated_expression,
                );
                continue;
            };
            if crate::ayy_env_flag!("AYY_TRACE_RUNTIME_SHADOWS") {
                eprintln!(
                    "runtime_shadow_param_writeback_sync_commit param={param_name} source_owner={source_owner} updated_expression={updated_expression:?}",
                    param_name = param_name,
                    source_owner = source_owner,
                );
            }
            self.sync_runtime_object_shadow_owner_static_metadata_from_expression(
                param_name,
                updated_expression,
            );
            self.sync_runtime_object_shadow_owner_static_metadata_from_expression(
                source_owner,
                updated_expression,
            );
        }
    }

    fn runtime_shadow_copy_effective_owner_name(&self, owner_name: &str) -> String {
        if owner_name == "this" {
            return owner_name.to_string();
        }

        let mut candidates = Vec::new();
        if let Some((resolved_name, _)) = self.resolve_current_local_binding(owner_name) {
            candidates.push(resolved_name);
        }
        if let Some(Expression::Identifier(source_name)) = self
            .state
            .speculation
            .static_semantics
            .local_value_binding(owner_name)
            .or_else(|| self.global_value_binding(owner_name))
        {
            candidates.push(source_name.clone());
        }
        if let Some(Expression::Identifier(source_name)) =
            self.resolve_bound_alias_expression(&Expression::Identifier(owner_name.to_string()))
        {
            candidates.push(source_name);
        }

        candidates
            .into_iter()
            .find(|candidate| {
                candidate != owner_name
                    && (self
                        .state
                        .speculation
                        .static_semantics
                        .has_local_object_binding(candidate)
                        || self.global_object_binding(candidate).is_some()
                        || self.runtime_object_property_shadow_owner_has_bindings(candidate))
            })
            .unwrap_or_else(|| owner_name.to_string())
    }

    pub(in crate::backend::direct_wasm) fn emit_runtime_object_property_shadow_copy(
        &mut self,
        source_owner: &str,
        target_owner: &str,
    ) -> DirectResult<()> {
        self.emit_runtime_object_property_shadow_copy_with_target_resolution(
            source_owner,
            target_owner,
            true,
            true,
            None,
        )
    }

    pub(in crate::backend::direct_wasm) fn emit_runtime_object_property_shadow_copy_to_exact_target(
        &mut self,
        source_owner: &str,
        target_owner: &str,
    ) -> DirectResult<()> {
        self.emit_runtime_object_property_shadow_copy_with_target_resolution(
            source_owner,
            target_owner,
            true,
            false,
            None,
        )
    }

    pub(in crate::backend::direct_wasm) fn emit_runtime_object_property_shadow_copy_between_exact_owners(
        &mut self,
        source_owner: &str,
        target_owner: &str,
    ) -> DirectResult<()> {
        self.emit_runtime_object_property_shadow_copy_with_target_resolution(
            source_owner,
            target_owner,
            false,
            false,
            None,
        )
    }

    fn emit_guarded_runtime_member_shadow_alias_property_copy(
        &mut self,
        source_owner: &str,
        target_owner: &str,
        property: &Expression,
        guard: &RuntimeMemberShadowAliasGuard,
    ) -> DirectResult<()> {
        let owner_binding = self.runtime_object_property_shadow_binding_by_property(
            &guard.parent_owner,
            &guard.parent_property,
        );

        self.push_global_get(owner_binding.present_index);
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();

        self.push_global_get(owner_binding.value_index);
        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        self.state.emission.output.instructions.push(0x46);
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();

        self.emit_runtime_member_shadow_alias_nullish_slot_condition(
            target_owner,
            &guard.assigned_property,
        );
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();

        self.emit_runtime_object_property_shadow_property_copy_between_exact_owners(
            source_owner,
            target_owner,
            property,
        )?;

        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }

    fn emit_runtime_member_shadow_alias_nullish_slot_condition(
        &mut self,
        owner_name: &str,
        property: &Expression,
    ) {
        let slot_binding =
            self.runtime_object_property_shadow_binding_by_property(owner_name, property);

        self.push_global_get(slot_binding.present_index);
        self.state.emission.output.instructions.push(0x45);
        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();
        self.push_i32_const(1);
        self.state.emission.output.instructions.push(0x05);

        self.push_global_get(slot_binding.value_index);
        self.push_i32_const(JS_NULL_TAG);
        self.state.emission.output.instructions.push(0x46);
        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();
        self.push_i32_const(1);
        self.state.emission.output.instructions.push(0x05);
        self.push_global_get(slot_binding.value_index);
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.state.emission.output.instructions.push(0x46);
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();

        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
    }

    fn emit_runtime_object_property_shadow_property_copy_between_exact_owners(
        &mut self,
        source_owner: &str,
        target_owner: &str,
        property: &Expression,
    ) -> DirectResult<()> {
        self.emit_runtime_object_property_shadow_copy_with_target_resolution(
            source_owner,
            target_owner,
            false,
            false,
            Some(property),
        )
    }

    fn emit_runtime_object_property_shadow_copy_with_target_resolution(
        &mut self,
        source_owner: &str,
        target_owner: &str,
        resolve_source_owner: bool,
        resolve_target_owner: bool,
        property_filter: Option<&Expression>,
    ) -> DirectResult<()> {
        let effective_source_owner = if resolve_source_owner {
            self.runtime_shadow_copy_effective_owner_name(source_owner)
        } else {
            source_owner.to_string()
        };
        let effective_target_owner = if resolve_target_owner {
            self.runtime_shadow_copy_effective_owner_name(target_owner)
        } else {
            target_owner.to_string()
        };
        let source_owner = effective_source_owner.as_str();
        let target_owner = effective_target_owner.as_str();
        if source_owner == target_owner {
            return Ok(());
        }
        if crate::ayy_env_flag!("AYY_TRACE_RUNTIME_SHADOWS") {
            eprintln!("runtime_shadow_copy {source_owner} -> {target_owner}");
        }
        let mut copy_entries = self.runtime_object_property_shadow_copy_entries(source_owner);
        if let Some(property_filter) = property_filter {
            let property_filter = self.canonical_runtime_shadow_property_expression(
                &self
                    .resolve_property_key_expression(property_filter)
                    .unwrap_or_else(|| self.materialize_static_expression(property_filter)),
            );
            copy_entries.retain(|(property, _)| {
                static_expression_matches(
                    &self.canonical_runtime_shadow_property_expression(property),
                    &property_filter,
                )
            });
        } else {
            self.append_target_private_runtime_shadow_copy_entries(
                source_owner,
                target_owner,
                &mut copy_entries,
            );
        }
        copy_entries.sort_by(|(left_property, _), (right_property, _)| {
            let left_refreshes_source_owner =
                Self::runtime_object_member_shadow_owner_name(target_owner, left_property)
                    == source_owner;
            let right_refreshes_source_owner =
                Self::runtime_object_member_shadow_owner_name(target_owner, right_property)
                    == source_owner;
            left_refreshes_source_owner.cmp(&right_refreshes_source_owner)
        });
        if crate::ayy_env_flag!("AYY_TRACE_RUNTIME_SHADOWS") {
            let entry_count = copy_entries.len();
            eprintln!(
                "runtime_shadow_copy_entries {source_owner}->{target_owner} count={entry_count}"
            );
        }
        for (property, mut fallback_value) in copy_entries {
            if Self::runtime_shadow_owner_is_class_object(target_owner)
                && matches!(
                    &property,
                    Expression::String(property_name)
                        if property_name.starts_with("__ayy$private$")
                )
            {
                continue;
            }
            let is_private_property = matches!(
                &property,
                Expression::String(property_name) if property_name.starts_with("__ayy$private$")
            );
            let is_private_shadow_property = matches!(
                &property,
                Expression::String(property_name)
                    if property_name.starts_with("__ayy$private$")
                        || property_name.starts_with("__ayy$private_brand$")
            );
            if crate::ayy_env_flag!("AYY_TRACE_RUNTIME_SHADOWS") {
                eprintln!(
                    "runtime_shadow_copy_entry {source_owner}->{target_owner} property={property:?} fallback={fallback_value:?} private={is_private_property}",
                );
            }
            let source_binding =
                self.runtime_object_property_shadow_binding_by_property(source_owner, &property);
            let target_binding =
                self.runtime_object_property_shadow_binding_by_property(target_owner, &property);
            let source_deleted = self.runtime_object_property_shadow_deleted_binding_by_property(
                source_owner,
                &property,
            );
            let target_deleted = self.runtime_object_property_shadow_deleted_binding_by_property(
                target_owner,
                &property,
            );
            let shadow_key = Self::runtime_object_property_shadow_key(&property);
            let source_shadow_name = format!("__ayy_object_property__{source_owner}__{shadow_key}");
            let target_shadow_name = format!("__ayy_object_property__{target_owner}__{shadow_key}");
            let source_member_owner =
                Self::runtime_object_member_shadow_owner_name(source_owner, &property);
            let target_member_owner =
                Self::runtime_object_member_shadow_owner_name(target_owner, &property);
            let mut force_object_fallback_value = false;
            let fallback_may_be_object = fallback_value.as_ref().is_some_and(|fallback_value| {
                !matches!(fallback_value, Expression::Null | Expression::Undefined)
                    && expression_may_evaluate_to_runtime_shadow_owner(fallback_value)
            });
            let fallback_aliases_copied_owner = fallback_value
                .as_ref()
                .and_then(|fallback_value| self.runtime_shadow_static_value_owner(fallback_value))
                .is_some_and(|owner| owner == source_owner || owner == target_owner);
            if fallback_may_be_object
                && self
                    .resolve_runtime_shadow_object_binding(&source_member_owner)
                    .as_ref()
                    .is_some_and(|binding| {
                        !self
                            .object_runtime_shadow_entries_from_binding(binding)
                            .is_empty()
                    })
            {
                fallback_value = Some(Expression::Identifier(
                    if fallback_aliases_copied_owner {
                        target_owner
                    } else {
                        target_member_owner.as_str()
                    }
                    .to_string(),
                ));
                force_object_fallback_value = true;
            }
            if !fallback_may_be_object
                && self
                    .resolve_runtime_shadow_object_binding(&source_member_owner)
                    .as_ref()
                    .is_some_and(|binding| {
                        !self
                            .object_runtime_shadow_entries_from_binding(binding)
                            .is_empty()
                    })
            {
                self.clear_runtime_object_property_shadow_static_metadata_prefix(
                    &target_member_owner,
                );
            }
            if let Some(descriptor_state) = self
                .backend
                .global_property_descriptor(&source_shadow_name)
                .cloned()
                .or_else(|| {
                    self.backend
                        .shared_global_semantics
                        .values
                        .property_descriptor(&source_shadow_name)
                        .cloned()
                })
            {
                if descriptor_state.has_get
                    || descriptor_state.has_set
                    || descriptor_state.getter.is_some()
                    || descriptor_state.setter.is_some()
                {
                    fallback_value = None;
                }
                self.backend.upsert_global_property_descriptor(
                    target_shadow_name.clone(),
                    descriptor_state.clone(),
                );
                self.backend
                    .shared_global_semantics
                    .values
                    .upsert_property_descriptor(target_shadow_name.clone(), descriptor_state);
            }
            if let Some(fallback_value) = fallback_value.as_ref() {
                let materialized_value =
                    self.reference_preserving_static_value_expression(fallback_value);
                self.update_static_global_assignment_metadata(
                    &target_shadow_name,
                    &materialized_value,
                );
            }
            if let Some(targets) = std::env::var_os("AYY_TRACE_GLOBAL_SET") {
                let targets = targets.to_string_lossy();
                let matches_target = targets
                    .split(',')
                    .filter_map(|target| target.trim().parse::<u32>().ok())
                    .any(|target| {
                        target == target_binding.value_index
                            || target == target_binding.present_index
                            || target == target_deleted.value_index
                            || target == target_deleted.present_index
                    });
                if matches_target {
                    eprintln!(
                        "runtime_shadow_copy_target_trace {source_owner}->{target_owner} property={property:?} fallback={fallback_value:?} target_binding=({}, {}) target_deleted=({}, {})",
                        target_binding.value_index,
                        target_binding.present_index,
                        target_deleted.value_index,
                        target_deleted.present_index,
                    );
                }
            }
            self.push_global_get(source_deleted.present_index);
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(target_binding.value_index);
            self.push_i32_const(0);
            self.push_global_set(target_binding.present_index);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(target_deleted.value_index);
            self.push_i32_const(1);
            self.push_global_set(target_deleted.present_index);
            self.emit_clear_runtime_member_value_shadow(
                source_owner,
                target_owner,
                &property,
                fallback_value.as_ref(),
            );
            self.state.emission.output.instructions.push(0x05);
            self.push_global_get(source_binding.present_index);
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(target_deleted.value_index);
            self.push_i32_const(0);
            self.push_global_set(target_deleted.present_index);
            if crate::ayy_env_flag!("AYY_TRACE_PRIVATE_MEMBER_VALUES") && is_private_shadow_property
            {
                let copied_value_local = self.allocate_temp_local();
                if force_object_fallback_value {
                    self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
                } else {
                    self.push_global_get(source_binding.value_index);
                }
                self.push_local_set(copied_value_local);
                self.emit_runtime_shadow_debug_print_local(
                    &format!(
                        "private_shadow_copy {source_owner}->{target_owner} {}",
                        static_property_name_from_expression(&property)
                            .unwrap_or_else(|| format!("{property:?}"))
                    ),
                    copied_value_local,
                )?;
                self.push_local_get(copied_value_local);
            } else if force_object_fallback_value {
                self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            } else {
                self.push_global_get(source_binding.value_index);
            }
            self.push_global_set(target_binding.value_index);
            self.push_i32_const(1);
            self.push_global_set(target_binding.present_index);
            self.emit_refresh_runtime_member_value_shadow(
                source_owner,
                target_owner,
                &property,
                fallback_value.as_ref(),
            )?;
            self.state.emission.output.instructions.push(0x05);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(target_deleted.value_index);
            self.push_i32_const(0);
            self.push_global_set(target_deleted.present_index);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(target_binding.value_index);
            if let Some(fallback_value) = fallback_value.as_ref() {
                if let Some(marker_fallback) = self.private_brand_marker_copy_fallback_for_target(
                    target_owner,
                    &property,
                    fallback_value,
                ) {
                    self.emit_runtime_shadow_fallback_value(&marker_fallback)?;
                } else if is_private_property
                    && !self.emit_private_brand_marker_runtime_value(
                        &Expression::Identifier(target_owner.to_string()),
                        &property,
                        fallback_value,
                    )?
                {
                    self.emit_runtime_shadow_fallback_value(fallback_value)?;
                } else if !is_private_property {
                    self.emit_runtime_shadow_fallback_value(fallback_value)?;
                }
                if crate::ayy_env_flag!("AYY_TRACE_PRIVATE_MEMBER_VALUES")
                    && is_private_shadow_property
                {
                    let copied_value_local = self.allocate_temp_local();
                    self.push_local_set(copied_value_local);
                    self.emit_runtime_shadow_debug_print_local(
                        &format!(
                            "private_shadow_seed {source_owner}->{target_owner} {}",
                            static_property_name_from_expression(&property)
                                .unwrap_or_else(|| format!("{property:?}"))
                        ),
                        copied_value_local,
                    )?;
                    self.push_local_get(copied_value_local);
                }
                self.push_global_set(target_binding.value_index);
                self.push_i32_const(1);
                self.push_global_set(target_binding.present_index);
                self.emit_refresh_runtime_member_value_shadow(
                    source_owner,
                    target_owner,
                    &property,
                    Some(fallback_value),
                )?;
            } else {
                if crate::ayy_env_flag!("AYY_TRACE_PRIVATE_MEMBER_VALUES")
                    && is_private_shadow_property
                {
                    self.emit_print(&[Expression::String(format!(
                        "private_shadow_absent {source_owner}->{target_owner} {}",
                        static_property_name_from_expression(&property)
                            .unwrap_or_else(|| format!("{property:?}"))
                    ))])?;
                }
                self.push_i32_const(0);
                self.push_global_set(target_binding.present_index);
                self.emit_clear_runtime_member_value_shadow(
                    source_owner,
                    target_owner,
                    &property,
                    None,
                );
            }
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }
        if property_filter.is_none() {
            self.emit_runtime_object_property_shadow_prefix_copy(source_owner, target_owner)?;
        }
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_object_spread_copy_data_properties_effects(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<()> {
        if !inline_summary_side_effect_free_expression(expression) {
            return Ok(());
        }
        let Some(object_binding) = self.resolve_object_binding_from_expression(expression) else {
            return Ok(());
        };

        for property_name in ordered_object_property_names(&object_binding) {
            if object_binding
                .non_enumerable_string_properties
                .iter()
                .any(|hidden_name| hidden_name == &property_name)
            {
                continue;
            }
            self.emit_member_read_without_prelude(expression, &Expression::String(property_name))?;
            self.state.emission.output.instructions.push(0x1a);
        }
        for (property, _) in &object_binding.symbol_properties {
            let getter_binding = self
                .resolve_member_getter_binding(expression, property)
                .or_else(|| {
                    object_binding_lookup_descriptor(&object_binding, property)
                        .and_then(|descriptor| descriptor.getter.as_ref())
                        .and_then(|getter| self.resolve_function_binding_from_expression(getter))
                });
            if let Some(LocalFunctionBinding::User(function_name)) = getter_binding {
                let capture_slots =
                    self.resolve_member_function_capture_slots(expression, property);
                self.emit_member_getter_call_with_bound_this(
                    &function_name,
                    expression,
                    capture_slots.as_ref(),
                )?;
            } else {
                self.emit_member_read_without_prelude(expression, property)?;
            }
            self.state.emission.output.instructions.push(0x1a);
        }

        Ok(())
    }
}
