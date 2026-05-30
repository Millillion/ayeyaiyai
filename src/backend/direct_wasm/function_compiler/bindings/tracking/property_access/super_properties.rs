use super::*;

impl<'a> FunctionCompiler<'a> {
    fn super_function_references_nested_function(
        function: &FunctionDeclaration,
        nested_function_name: &str,
    ) -> bool {
        if collect_referenced_binding_names_from_statements(&function.body)
            .contains(nested_function_name)
        {
            return true;
        }
        function.params.iter().any(|parameter| {
            parameter.default.as_ref().is_some_and(|default| {
                let mut referenced = HashSet::new();
                collect_referenced_binding_names_from_expression(default, &mut referenced);
                referenced.contains(nested_function_name)
            })
        })
    }

    fn lexical_enclosing_function_name(&self, function_name: &str) -> Option<String> {
        let function = self
            .backend
            .function_registry
            .catalog
            .registered_function(function_name)?;
        if !function.lexical_this {
            return None;
        }
        self.backend
            .function_registry
            .catalog
            .registered_function_declarations
            .iter()
            .find(|candidate| {
                candidate.name != function_name
                    && Self::super_function_references_nested_function(candidate, function_name)
            })
            .map(|candidate| candidate.name.clone())
    }

    fn resolve_home_object_name_for_function_inner(
        &self,
        function_name: &str,
        visited: &mut HashSet<String>,
    ) -> Option<String> {
        if !visited.insert(function_name.to_string()) {
            return None;
        }
        if let Some(home_object_name) = self
            .user_function(function_name)?
            .home_object_binding
            .as_ref()
        {
            return Some(home_object_name.clone());
        }
        if let Some(home_object_name) = self.find_global_home_object_binding_name(function_name) {
            return Some(home_object_name);
        }
        let enclosing_function_name = self.lexical_enclosing_function_name(function_name)?;
        self.resolve_home_object_name_for_function_inner(&enclosing_function_name, visited)
    }

    pub(in crate::backend::direct_wasm) fn resolve_home_object_name_for_function(
        &self,
        function_name: &str,
    ) -> Option<String> {
        self.resolve_home_object_name_for_function_inner(function_name, &mut HashSet::new())
    }

    pub(in crate::backend::direct_wasm) fn resolve_super_base_expression_with_context(
        &self,
        current_function_name: Option<&str>,
    ) -> Option<Expression> {
        let trace_super_resolution = std::env::var_os("AYY_TRACE_SUPER_RESOLUTION").is_some();
        let function_name = current_function_name?;
        if let Some(function) = self.resolve_registered_function_declaration(function_name)
            && function.derived_constructor
            && let Some(self_binding) = function.self_binding.as_deref()
            && let Some(super_constructor) = self
                .global_object_prototype_expression(self_binding)
                .cloned()
        {
            let materialized_super_constructor = match &super_constructor {
                Expression::Identifier(name) => self
                    .resolve_static_class_init_local_alias_expression(name)
                    .or_else(|| {
                        self.global_value_binding(name)
                            .filter(|resolved| {
                                !static_expression_matches(resolved, &super_constructor)
                            })
                            .cloned()
                    })
                    .filter(|resolved| !static_expression_matches(resolved, &super_constructor))
                    .map(|resolved| self.materialize_static_expression(&resolved))
                    .unwrap_or_else(|| self.materialize_static_expression(&super_constructor)),
                _ => self.materialize_static_expression(&super_constructor),
            };
            let resolved = match materialized_super_constructor {
                Expression::Identifier(name) => Some(Self::prototype_member_expression(&name)),
                _ => Some(Expression::Member {
                    object: Box::new(materialized_super_constructor),
                    property: Box::new(Expression::String("prototype".to_string())),
                }),
            };
            if trace_super_resolution {
                eprintln!(
                    "super_resolution:base current={current_function_name:?} constructor self={self_binding:?} resolved={resolved:?}"
                );
            }
            return resolved;
        }
        let home_object_name = match self.resolve_home_object_name_for_function(function_name) {
            Some(home_object_name) => home_object_name,
            None => {
                if trace_super_resolution {
                    eprintln!("super_resolution:base current={current_function_name:?} home=None");
                }
                let enclosing_function_name =
                    self.lexical_enclosing_function_name(function_name)?;
                return self
                    .resolve_super_base_expression_with_context(Some(&enclosing_function_name));
            }
        };
        let resolved = self
            .global_object_prototype_expression(&home_object_name)
            .cloned();
        if trace_super_resolution {
            eprintln!(
                "super_resolution:base current={current_function_name:?} home={home_object_name:?} resolved={resolved:?}"
            );
        }
        resolved
    }

    pub(in crate::backend::direct_wasm) fn resolve_super_runtime_prototype_binding_with_context(
        &self,
        current_function_name: Option<&str>,
    ) -> Option<(String, GlobalObjectRuntimePrototypeBinding)> {
        let function_name = current_function_name?;
        let home_object_name = self.resolve_home_object_name_for_function(function_name)?;
        let binding = self
            .global_runtime_prototype_binding(&home_object_name)?
            .clone();
        Some((home_object_name, binding))
    }

    pub(in crate::backend::direct_wasm) fn emit_runtime_super_property_value_from_base(
        &mut self,
        base: Option<&Expression>,
        property: &Expression,
    ) -> DirectResult<()> {
        let Some(base) = base else {
            self.emit_named_error_throw("TypeError")?;
            return Ok(());
        };
        if let Some(function_binding) = self.resolve_member_function_binding(base, property) {
            match function_binding {
                LocalFunctionBinding::User(function_name) => {
                    if let Some(user_function) = self.user_function(&function_name) {
                        self.push_i32_const(user_function_runtime_value(user_function));
                    } else {
                        self.push_i32_const(JS_UNDEFINED_TAG);
                    }
                }
                LocalFunctionBinding::Builtin(_) => {
                    self.push_i32_const(JS_TYPEOF_FUNCTION_TAG);
                }
            }
            return Ok(());
        }
        if let Some(function_binding) = self
            .resolve_member_getter_binding_shallow(base, property)
            .or_else(|| self.resolve_member_getter_binding(base, property))
        {
            match function_binding {
                LocalFunctionBinding::User(function_name) => {
                    let capture_slots = self.resolve_member_function_capture_slots(base, property);
                    self.emit_member_getter_call_with_bound_this(
                        &function_name,
                        &Expression::This,
                        capture_slots.as_ref(),
                    )?;
                }
                LocalFunctionBinding::Builtin(function_name) => {
                    let callee = Expression::Identifier(function_name);
                    if !self.emit_arguments_slot_accessor_call(&callee, &[], 0, Some(&[]))? {
                        self.push_i32_const(JS_UNDEFINED_TAG);
                    }
                }
            }
            return Ok(());
        }
        let materialized_property = self.materialize_static_expression(property);
        if let Some(object_binding) = self.resolve_object_binding_from_expression(base)
            && let Some(value) = self.resolve_object_binding_property_value_for_object(
                base,
                &object_binding,
                &materialized_property,
            )
        {
            self.emit_numeric_expression(&value)?;
            return Ok(());
        }
        if let Some(value) =
            self.resolve_inherited_object_property_value(base, &materialized_property)
        {
            self.emit_numeric_expression(&value)?;
            return Ok(());
        }
        self.push_i32_const(JS_UNDEFINED_TAG);
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_super_member_read_via_runtime_prototype_binding(
        &mut self,
        property: &Expression,
    ) -> DirectResult<bool> {
        let Some((_, binding)) =
            self.resolve_super_runtime_prototype_binding_with_context(self.current_function_name())
        else {
            return Ok(false);
        };
        let Some(global_index) = binding.global_index else {
            return Ok(false);
        };
        let resolved_property = self
            .resolve_property_key_expression(property)
            .unwrap_or_else(|| self.materialize_static_expression(property));
        if !matches!(
            resolved_property,
            Expression::String(_) | Expression::Identifier(_) | Expression::Member { .. }
        ) {
            return Ok(false);
        }

        let state_local = self.allocate_temp_local();
        self.push_global_get(global_index);
        self.push_local_set(state_local);

        let mut open_frames = 0;
        for (variant_index, prototype) in binding.variants.iter().enumerate() {
            self.push_local_get(state_local);
            self.push_i32_const(variant_index as i32);
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            open_frames += 1;
            self.emit_runtime_super_property_value_from_base(
                prototype.as_ref(),
                &resolved_property,
            )?;
            self.state.emission.output.instructions.push(0x05);
        }

        self.push_i32_const(JS_UNDEFINED_TAG);
        for _ in 0..open_frames {
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn resolve_super_function_binding(
        &self,
        property: &Expression,
    ) -> Option<LocalFunctionBinding> {
        self.resolve_super_function_binding_with_context(property, self.current_function_name())
    }

    pub(in crate::backend::direct_wasm) fn resolve_super_function_binding_with_context(
        &self,
        property: &Expression,
        current_function_name: Option<&str>,
    ) -> Option<LocalFunctionBinding> {
        let base = self.resolve_super_base_expression_with_context(current_function_name)?;
        let materialized_property = self.materialize_static_expression(property);
        self.resolve_member_function_binding(&base, property)
            .or_else(|| {
                self.resolve_object_binding_from_expression(&base)
                    .and_then(|object_binding| {
                        object_binding_lookup_value(&object_binding, &materialized_property)
                            .cloned()
                    })
                    .and_then(|value| self.resolve_function_binding_from_expression(&value))
            })
    }

    pub(in crate::backend::direct_wasm) fn resolve_super_getter_binding(
        &self,
        property: &Expression,
    ) -> Option<LocalFunctionBinding> {
        self.resolve_super_getter_binding_with_context(property, self.current_function_name())
    }

    pub(in crate::backend::direct_wasm) fn resolve_super_getter_binding_with_context(
        &self,
        property: &Expression,
        current_function_name: Option<&str>,
    ) -> Option<LocalFunctionBinding> {
        let base = self.resolve_super_base_expression_with_context(current_function_name)?;
        self.resolve_member_getter_binding_shallow(&base, property)
            .or_else(|| self.resolve_member_getter_binding(&base, property))
    }

    pub(in crate::backend::direct_wasm) fn resolve_super_value_expression(
        &self,
        property: &Expression,
    ) -> Option<Expression> {
        self.resolve_super_value_expression_with_context(property, self.current_function_name())
    }

    pub(in crate::backend::direct_wasm) fn resolve_super_value_expression_with_context(
        &self,
        property: &Expression,
        current_function_name: Option<&str>,
    ) -> Option<Expression> {
        let trace_super_resolution = std::env::var_os("AYY_TRACE_SUPER_RESOLUTION").is_some();
        let base = self.resolve_super_base_expression_with_context(current_function_name)?;
        let materialized_property = self.materialize_static_expression(property);
        let resolved = self
            .resolve_object_binding_from_expression(&base)
            .and_then(|object_binding| {
                self.resolve_object_binding_property_value_for_object(
                    &base,
                    &object_binding,
                    &materialized_property,
                )
            })
            .or_else(|| {
                self.resolve_inherited_object_property_value(&base, &materialized_property)
            });
        if trace_super_resolution {
            eprintln!(
                "super_resolution:value current={current_function_name:?} base={base:?} property={materialized_property:?} resolved={resolved:?}"
            );
        }
        resolved
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_super_member_value_with_context(
        &self,
        property: &Expression,
        current_function_name: Option<&str>,
        receiver: &Expression,
    ) -> Option<Expression> {
        let materialized_property = self.materialize_static_expression(property);
        let super_base = self.resolve_super_base_expression_with_context(current_function_name);
        let super_function_binding = super_base.as_ref().and_then(|base| {
            self.resolve_member_function_binding(base, &materialized_property)
                .or_else(|| {
                    self.resolve_object_binding_from_expression(base)
                        .and_then(|object_binding| {
                            object_binding_lookup_value(&object_binding, &materialized_property)
                                .cloned()
                        })
                        .and_then(|value| self.resolve_function_binding_from_expression(&value))
                })
        });
        if super_function_binding.is_some() {
            return Some(Expression::Member {
                object: Box::new(super_base?),
                property: Box::new(materialized_property),
            });
        }
        if let Some(getter_binding) = self.resolve_super_getter_binding_with_context(
            &materialized_property,
            current_function_name,
        ) {
            return self.resolve_static_getter_value_from_binding_with_context(
                &getter_binding,
                receiver,
                current_function_name,
            );
        }
        self.resolve_super_value_expression_with_context(
            &materialized_property,
            current_function_name,
        )
    }
}
