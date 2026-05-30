use super::*;

thread_local! {
    static ACTIVE_PROXY_BINDING_RESOLUTION_SHAPES: RefCell<HashSet<String>> =
        RefCell::new(HashSet::new());
}

struct ProxyBindingResolutionShapeGuard {
    key: String,
}

impl ProxyBindingResolutionShapeGuard {
    fn enter(expression: &Expression) -> Option<Self> {
        let key = format!("{expression:?}");
        let inserted = ACTIVE_PROXY_BINDING_RESOLUTION_SHAPES
            .with(|active| active.borrow_mut().insert(key.clone()));
        inserted.then_some(Self { key })
    }
}

impl Drop for ProxyBindingResolutionShapeGuard {
    fn drop(&mut self) {
        ACTIVE_PROXY_BINDING_RESOLUTION_SHAPES.with(|active| {
            active.borrow_mut().remove(&self.key);
        });
    }
}

impl<'a> FunctionCompiler<'a> {
    fn resolve_proxy_member_binding_from_handler(
        &self,
        handler: &Expression,
        property_name: &str,
    ) -> Option<LocalFunctionBinding> {
        if let Expression::Call { callee, arguments } = handler
            && matches!(callee.as_ref(), Expression::Identifier(name) if name == "allowProxyTraps")
            && let Some(CallArgument::Expression(overrides) | CallArgument::Spread(overrides)) =
                arguments.first()
            && let Some(binding) =
                self.resolve_proxy_member_binding_from_handler(overrides, property_name)
        {
            return Some(binding);
        }

        if let Expression::Call { callee, arguments } = handler
            && let Some((resolved_handler, _)) = self
                .resolve_static_call_result_expression_with_context(
                    callee,
                    arguments,
                    self.current_function_name(),
                )
            && !static_expression_matches(&resolved_handler, handler)
            && let Some(binding) =
                self.resolve_proxy_member_binding_from_handler(&resolved_handler, property_name)
        {
            return Some(binding);
        }

        let property = Expression::String(property_name.to_string());
        match handler {
            Expression::Identifier(name) => {
                let key = MemberFunctionBindingKey {
                    target: MemberFunctionBindingTarget::Identifier(name.clone()),
                    property: MemberFunctionBindingProperty::String(property_name.to_string()),
                };
                self.member_function_binding_entry(&key).or_else(|| {
                    self.resolve_object_binding_from_expression(handler)
                        .and_then(|object_binding| {
                            object_binding_lookup_value(&object_binding, &property).and_then(
                                |value| self.resolve_function_binding_from_expression(value),
                            )
                        })
                })
            }
            Expression::Object(entries) => entries.iter().find_map(|entry| {
                let crate::ir::hir::ObjectEntry::Data { key, value } = entry else {
                    return None;
                };
                let key = self
                    .resolve_property_key_expression(key)
                    .unwrap_or_else(|| self.materialize_static_expression(key));
                if !matches!(key, Expression::String(ref name) if name == property_name) {
                    return None;
                }
                self.resolve_function_binding_from_expression(value)
            }),
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_proxy_has_binding_from_handler(
        &self,
        handler: &Expression,
    ) -> Option<LocalFunctionBinding> {
        self.resolve_proxy_member_binding_from_handler(handler, "has")
    }

    pub(in crate::backend::direct_wasm) fn resolve_proxy_get_binding_from_handler(
        &self,
        handler: &Expression,
    ) -> Option<LocalFunctionBinding> {
        self.resolve_proxy_member_binding_from_handler(handler, "get")
    }

    pub(in crate::backend::direct_wasm) fn resolve_proxy_set_binding_from_handler(
        &self,
        handler: &Expression,
    ) -> Option<LocalFunctionBinding> {
        self.resolve_proxy_member_binding_from_handler(handler, "set")
    }

    pub(in crate::backend::direct_wasm) fn resolve_proxy_get_own_property_descriptor_binding_from_handler(
        &self,
        handler: &Expression,
    ) -> Option<LocalFunctionBinding> {
        self.resolve_proxy_member_binding_from_handler(handler, "getOwnPropertyDescriptor")
    }

    pub(in crate::backend::direct_wasm) fn resolve_proxy_define_property_binding_from_handler(
        &self,
        handler: &Expression,
    ) -> Option<LocalFunctionBinding> {
        self.resolve_proxy_member_binding_from_handler(handler, "defineProperty")
    }

    pub(in crate::backend::direct_wasm) fn resolve_proxy_own_keys_binding_from_handler(
        &self,
        handler: &Expression,
    ) -> Option<LocalFunctionBinding> {
        self.resolve_proxy_member_binding_from_handler(handler, "ownKeys")
    }

    pub(in crate::backend::direct_wasm) fn resolve_proxy_binding_from_expression(
        &self,
        expression: &Expression,
    ) -> Option<ProxyValueBinding> {
        let _guard = ProxyBindingResolutionShapeGuard::enter(expression)?;
        match expression {
            Expression::This => self
                .state
                .speculation
                .static_semantics
                .local_proxy_binding("this")
                .cloned()
                .or_else(|| {
                    self.state
                        .speculation
                        .static_semantics
                        .local_value_binding("this")
                        .and_then(|value| {
                            (!matches!(value, Expression::Undefined | Expression::This))
                                .then(|| self.resolve_proxy_binding_from_expression(value))
                                .flatten()
                        })
                }),
            Expression::Identifier(name) => self
                .state
                .speculation
                .static_semantics
                .local_proxy_binding(name)
                .cloned()
                .or_else(|| self.backend.global_proxy_binding(name).cloned())
                .or_else(|| {
                    self.state
                        .speculation
                        .static_semantics
                        .local_value_binding(name)
                        .and_then(|value| {
                            (!static_expression_matches(value, expression))
                                .then(|| self.resolve_proxy_binding_from_expression(value))
                                .flatten()
                        })
                })
                .or_else(|| {
                    self.global_value_binding(name).and_then(|value| {
                        (!static_expression_matches(value, expression))
                            .then(|| self.resolve_proxy_binding_from_expression(value))
                            .flatten()
                    })
                }),
            Expression::New { callee, arguments } if matches!(callee.as_ref(), Expression::Identifier(name) if name == "Proxy" && self.is_unshadowed_builtin_identifier(name)) =>
            {
                let [
                    CallArgument::Expression(target),
                    CallArgument::Expression(handler),
                    ..,
                ] = arguments.as_slice()
                else {
                    return None;
                };
                Some(ProxyValueBinding {
                    target: self.materialize_static_expression(target),
                    handler: self.materialize_static_expression(handler),
                    get_binding: self.resolve_proxy_get_binding_from_handler(handler),
                    has_binding: self.resolve_proxy_has_binding_from_handler(handler),
                    set_binding: self.resolve_proxy_set_binding_from_handler(handler),
                    get_own_property_descriptor_binding: self
                        .resolve_proxy_get_own_property_descriptor_binding_from_handler(handler),
                    define_property_binding: self
                        .resolve_proxy_define_property_binding_from_handler(handler),
                    own_keys_binding: self.resolve_proxy_own_keys_binding_from_handler(handler),
                })
            }
            Expression::New { callee, arguments } => {
                let LocalFunctionBinding::User(function_name) =
                    self.resolve_function_binding_from_expression(callee)?
                else {
                    return None;
                };
                let user_function = self.user_function(&function_name)?;
                if !user_function.is_constructible() {
                    return None;
                }
                let capture_source_bindings =
                    self.resolve_constructor_capture_source_bindings_from_expression(callee);
                let return_expression = self
                    .resolve_user_constructor_return_expression_for_function(
                        user_function,
                        arguments,
                        capture_source_bindings.as_ref(),
                    )?;
                (!static_expression_matches(&return_expression, expression))
                    .then(|| self.resolve_proxy_binding_from_expression(&return_expression))
                    .flatten()
            }
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn update_local_proxy_binding(
        &mut self,
        name: &str,
        value: &Expression,
    ) {
        if matches!(
            value,
            Expression::New { callee, .. }
                if matches!(
                    callee.as_ref(),
                    Expression::Identifier(name) if !name.starts_with("__ayy_class_ctor_")
                )
        ) {
            self.state
                .speculation
                .static_semantics
                .clear_local_proxy_binding(name);
            if self.binding_name_is_global(name) {
                self.backend.sync_global_proxy_binding(name, None);
            }
            return;
        }
        let Some(proxy_binding) = self.resolve_proxy_binding_from_expression(value) else {
            self.state
                .speculation
                .static_semantics
                .clear_local_proxy_binding(name);
            if self.binding_name_is_global(name) {
                self.backend.sync_global_proxy_binding(name, None);
            }
            return;
        };
        self.state
            .speculation
            .static_semantics
            .set_local_proxy_binding(name, proxy_binding.clone());
        if self.binding_name_is_global(name) {
            self.backend
                .sync_global_proxy_binding(name, Some(proxy_binding));
        }
        self.state
            .speculation
            .static_semantics
            .set_local_kind(name, StaticValueKind::Object);
    }
}
