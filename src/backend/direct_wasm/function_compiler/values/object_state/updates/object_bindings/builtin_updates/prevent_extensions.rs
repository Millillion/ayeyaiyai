use super::*;

impl<'a> FunctionCompiler<'a> {
    fn resolve_object_binding_storage_name(&self, name: &str) -> String {
        if name == "this" || name == Self::STATIC_NEW_THIS_BINDING {
            return name.to_string();
        }
        self.resolve_current_local_binding(name)
            .map(|(resolved_name, _)| resolved_name)
            .unwrap_or_else(|| name.to_string())
    }

    fn resolve_object_binding_update_target_name(&self, expression: &Expression) -> Option<String> {
        match expression {
            Expression::Identifier(name) => Some(name.clone()),
            Expression::Member { object, property } if matches!(property.as_ref(), Expression::String(name) if name == "prototype") => {
                self.resolve_object_binding_update_target_name(object)
                    .map(|name| format!("{name}.prototype"))
            }
            Expression::This => Some("this".to_string()),
            _ => self
                .resolve_bound_alias_expression(expression)
                .filter(|resolved| !static_expression_matches(resolved, expression))
                .as_ref()
                .and_then(|resolved| self.resolve_object_binding_update_target_name(resolved)),
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_object_extensibility(
        &self,
        expression: &Expression,
    ) -> Option<bool> {
        let direct_lookup = match expression {
            Expression::Identifier(name) => {
                let storage_name = self.resolve_object_binding_storage_name(name);
                self.state
                    .speculation
                    .static_semantics
                    .local_object_binding(&storage_name)
                    .cloned()
                    .or_else(|| {
                        self.backend
                            .global_semantics
                            .values
                            .object_binding(name)
                            .cloned()
                    })
                    .or_else(|| {
                        self.state
                            .speculation
                            .static_semantics
                            .objects
                            .local_prototype_object_bindings
                            .get(&storage_name)
                            .cloned()
                    })
                    .or_else(|| {
                        self.backend
                            .global_semantics
                            .values
                            .prototype_object_bindings
                            .get(name)
                            .cloned()
                    })
            }
            Expression::This => self
                .state
                .speculation
                .static_semantics
                .local_object_binding("this")
                .cloned(),
            _ => None,
        };

        direct_lookup
            .or_else(|| self.resolve_object_binding_from_expression(expression))
            .map(|binding| binding.extensible)
            .or_else(|| {
                self.resolve_static_object_prototype_expression(expression)
                    .map(|_| true)
            })
    }

    fn prevent_extensions_on_binding_name(&mut self, name: &str) -> bool {
        if self.current_function_name().is_none() && (name == "this" || name == "globalThis") {
            let mut binding = self
                .backend
                .global_object_binding(name)
                .cloned()
                .unwrap_or_else(empty_object_value_binding);
            object_binding_prevent_extensions(&mut binding);
            self.backend
                .global_semantics
                .values
                .sync_object_binding(name, Some(binding.clone()));
            self.backend
                .shared_global_semantics
                .values
                .sync_object_binding(name, Some(binding));
            return true;
        }

        if let Some(prototype_owner_name) = name.strip_suffix(".prototype") {
            let mut prevented = false;
            if let Some(binding) = self
                .state
                .speculation
                .static_semantics
                .objects
                .local_prototype_object_bindings
                .get_mut(prototype_owner_name)
            {
                object_binding_prevent_extensions(binding);
                prevented = true;
            }
            if let Some(binding) = self
                .backend
                .global_semantics
                .values
                .prototype_object_bindings
                .get_mut(prototype_owner_name)
            {
                object_binding_prevent_extensions(binding);
                prevented = true;
            }
            if prevented {
                self.sync_current_this_binding_for_home_object_update(name, false);
            }
            return prevented;
        }

        let storage_name = self.resolve_object_binding_storage_name(name);
        if let Some(binding) = self
            .state
            .speculation
            .static_semantics
            .local_object_binding_mut(&storage_name)
        {
            object_binding_prevent_extensions(binding);
            return true;
        }
        if let Some(binding) = self
            .backend
            .global_semantics
            .values
            .object_binding_mut(name)
        {
            object_binding_prevent_extensions(binding);
            return true;
        }
        if let Some(binding) = self
            .state
            .speculation
            .static_semantics
            .objects
            .local_prototype_object_bindings
            .get_mut(&storage_name)
        {
            object_binding_prevent_extensions(binding);
            return true;
        }
        if let Some(binding) = self
            .backend
            .global_semantics
            .values
            .prototype_object_bindings
            .get_mut(name)
        {
            object_binding_prevent_extensions(binding);
            return true;
        }
        if let Some(mut binding) =
            self.resolve_object_binding_from_expression(&Expression::Identifier(name.to_string()))
        {
            object_binding_prevent_extensions(&mut binding);
            if self.binding_name_is_global(name) {
                self.backend
                    .global_semantics
                    .values
                    .object_bindings
                    .insert(name.to_string(), binding);
            } else {
                self.state
                    .speculation
                    .static_semantics
                    .set_local_object_binding(&storage_name, binding);
            }
            return true;
        }
        false
    }

    fn freeze_binding_name(&mut self, name: &str) -> bool {
        if self.current_function_name().is_none() && (name == "this" || name == "globalThis") {
            let mut binding = self
                .backend
                .global_object_binding(name)
                .cloned()
                .unwrap_or_else(empty_object_value_binding);
            object_binding_freeze(&mut binding);
            self.backend
                .global_semantics
                .values
                .sync_object_binding(name, Some(binding.clone()));
            self.backend
                .shared_global_semantics
                .values
                .sync_object_binding(name, Some(binding));
            return true;
        }

        if let Some(prototype_owner_name) = name.strip_suffix(".prototype") {
            let mut frozen = false;
            if let Some(binding) = self
                .state
                .speculation
                .static_semantics
                .objects
                .local_prototype_object_bindings
                .get_mut(prototype_owner_name)
            {
                object_binding_freeze(binding);
                frozen = true;
            }
            if let Some(binding) = self
                .backend
                .global_semantics
                .values
                .prototype_object_bindings
                .get_mut(prototype_owner_name)
            {
                object_binding_freeze(binding);
                frozen = true;
            }
            if frozen {
                self.sync_current_this_binding_for_home_object_update(name, true);
            }
            return frozen;
        }

        let storage_name = self.resolve_object_binding_storage_name(name);
        if let Some(binding) = self
            .state
            .speculation
            .static_semantics
            .local_object_binding_mut(&storage_name)
        {
            object_binding_freeze(binding);
            return true;
        }
        if let Some(binding) = self
            .backend
            .global_semantics
            .values
            .object_binding_mut(name)
        {
            object_binding_freeze(binding);
            return true;
        }
        if let Some(binding) = self
            .state
            .speculation
            .static_semantics
            .objects
            .local_prototype_object_bindings
            .get_mut(&storage_name)
        {
            object_binding_freeze(binding);
            return true;
        }
        if let Some(binding) = self
            .backend
            .global_semantics
            .values
            .prototype_object_bindings
            .get_mut(name)
        {
            object_binding_freeze(binding);
            return true;
        }
        if let Some(mut binding) =
            self.resolve_object_binding_from_expression(&Expression::Identifier(name.to_string()))
        {
            object_binding_freeze(&mut binding);
            if self.binding_name_is_global(name) {
                self.backend
                    .global_semantics
                    .values
                    .object_bindings
                    .insert(name.to_string(), binding);
            } else {
                self.state
                    .speculation
                    .static_semantics
                    .set_local_object_binding(&storage_name, binding);
            }
            return true;
        }
        false
    }

    fn sync_current_this_binding_for_home_object_update(
        &mut self,
        target_name: &str,
        freeze_after_merge: bool,
    ) {
        if !self
            .current_user_function()
            .is_some_and(|function| function.home_object_binding.as_deref() == Some(target_name))
        {
            return;
        }

        let binding = if let Some(prototype_owner_name) = target_name.strip_suffix(".prototype") {
            self.state
                .speculation
                .static_semantics
                .objects
                .local_prototype_object_bindings
                .get(prototype_owner_name)
                .cloned()
                .or_else(|| {
                    self.backend
                        .global_semantics
                        .values
                        .prototype_object_bindings
                        .get(prototype_owner_name)
                        .cloned()
                })
        } else {
            let storage_name = self.resolve_object_binding_storage_name(target_name);
            self.state
                .speculation
                .static_semantics
                .local_object_binding(&storage_name)
                .cloned()
                .or_else(|| {
                    self.backend
                        .global_semantics
                        .values
                        .object_binding(target_name)
                        .cloned()
                })
        };

        let Some(mut binding) = binding else {
            return;
        };
        if let Some(current_this_binding) = self
            .state
            .speculation
            .static_semantics
            .local_object_binding("this")
            .cloned()
        {
            let extensible = binding.extensible;
            Self::merge_object_binding_properties(&mut binding, &current_this_binding);
            binding.extensible = extensible;
        }
        if freeze_after_merge {
            object_binding_freeze(&mut binding);
        }

        self.state
            .speculation
            .static_semantics
            .set_local_object_binding("this", binding);
        self.state
            .speculation
            .static_semantics
            .set_local_kind("this", StaticValueKind::Object);
    }

    pub(in crate::backend::direct_wasm) fn apply_object_prevent_extensions_update(
        &mut self,
        callee_object: &Expression,
        arguments: &[CallArgument],
    ) {
        if !matches!(callee_object, Expression::Identifier(name) if name == "Object" || name == "Reflect")
        {
            return;
        }
        let Some(CallArgument::Expression(target) | CallArgument::Spread(target)) =
            arguments.first()
        else {
            return;
        };
        let Some(target_name) = self.resolve_object_binding_update_target_name(target) else {
            return;
        };
        self.prevent_extensions_on_binding_name(&target_name);
        self.sync_current_this_binding_for_home_object_update(&target_name, false);
    }

    pub(in crate::backend::direct_wasm) fn apply_object_freeze_update(
        &mut self,
        arguments: &[CallArgument],
    ) {
        let Some(CallArgument::Expression(target) | CallArgument::Spread(target)) =
            arguments.first()
        else {
            return;
        };
        let Some(target_name) = self.resolve_object_binding_update_target_name(target) else {
            return;
        };
        self.freeze_binding_name(&target_name);
        self.sync_current_this_binding_for_home_object_update(&target_name, true);
    }
}
