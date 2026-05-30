use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn emit_typeof_value_expression(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<()> {
        if let Some(text) = self
            .infer_typeof_operand_kind(expression)
            .and_then(StaticValueKind::as_typeof_str)
        {
            return self.emit_static_string_literal(text);
        }

        let type_tag_local = self.allocate_temp_local();
        self.emit_typeof_expression(expression)?;
        self.push_local_set(type_tag_local);
        self.emit_typeof_value_from_tag_local(type_tag_local)
    }

    pub(in crate::backend::direct_wasm) fn emit_typeof_value_from_tag_local(
        &mut self,
        type_tag_local: u32,
    ) -> DirectResult<()> {
        let result_local = self.allocate_temp_local();
        self.emit_static_string_literal("number")?;
        self.push_local_set(result_local);

        for (type_tag, text) in [
            (JS_TYPEOF_BOOLEAN_TAG, "boolean"),
            (JS_TYPEOF_STRING_TAG, "string"),
            (JS_TYPEOF_OBJECT_TAG, "object"),
            (JS_TYPEOF_UNDEFINED_TAG, "undefined"),
            (JS_TYPEOF_FUNCTION_TAG, "function"),
            (JS_TYPEOF_SYMBOL_TAG, "symbol"),
            (JS_TYPEOF_BIGINT_TAG, "bigint"),
        ] {
            self.push_local_get(type_tag_local);
            self.push_i32_const(type_tag);
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.emit_static_string_literal(text)?;
            self.push_local_set(result_local);
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }

        self.push_local_get(result_local);
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_typeof_expression(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<()> {
        let trace_typeof = std::env::var_os("AYY_TRACE_TYPEOF").is_some()
            || std::env::var_os("AYY_TRACE_ASSERTIONS").is_some();
        if trace_typeof {
            eprintln!("emit_typeof_expression:start expression={expression:?}");
        }
        if let Expression::Identifier(name) = expression
            && self
                .state
                .speculation
                .static_semantics
                .eval_lexical_initialized_locals
                .contains_key(name)
        {
            if trace_typeof {
                eprintln!("emit_typeof_expression:eval_lexical name={name}");
            }
            self.emit_eval_lexical_binding_read(name)?;
            self.state.emission.output.instructions.push(0x1a);
        }
        if let Expression::Identifier(name) = expression
            && self
                .resolve_current_local_binding(name)
                .and_then(|(resolved_name, _)| self.local_lexical_initialized_local(&resolved_name))
                .is_some()
        {
            if trace_typeof {
                eprintln!("emit_typeof_expression:local_lexical name={name}");
            }
            self.emit_runtime_typeof_tag(expression)?;
            return Ok(());
        }
        if let Expression::Identifier(name) = expression
            && self.resolve_current_local_binding(name).is_none()
            && !self
                .backend
                .global_semantics
                .names
                .bindings
                .contains_key(name)
            && self.emit_typeof_user_function_capture_binding(name)?
        {
            if trace_typeof {
                eprintln!("emit_typeof_expression:user_function_capture name={name}");
            }
            return Ok(());
        }
        if let Expression::Identifier(name) = expression
            && self.resolve_current_local_binding(name).is_none()
            && !self
                .backend
                .global_semantics
                .names
                .bindings
                .contains_key(name)
            && self.emit_typeof_eval_local_function_binding(name)?
        {
            if trace_typeof {
                eprintln!("emit_typeof_expression:eval_local_function name={name}");
            }
            return Ok(());
        }
        let skip_static_function_binding_probe = matches!(
            expression,
            Expression::Member { object, property }
                if matches!(
                    property.as_ref(),
                    Expression::String(name)
                        if matches!(
                            name.as_str(),
                            "value" | "configurable" | "enumerable" | "writable" | "get" | "set"
                        )
                )
                    && matches!(
                        object.as_ref(),
                        Expression::Identifier(name)
                            if self.local_binding_is_dynamic_property_descriptor_result(name)
                    )
        );
        if trace_typeof {
            eprintln!(
                "emit_typeof_expression:function_binding_probe skip={skip_static_function_binding_probe} expression={expression:?}"
            );
        }
        if !skip_static_function_binding_probe
            && self
                .resolve_function_binding_from_expression(expression)
                .is_some()
        {
            if trace_typeof {
                eprintln!("emit_typeof_expression:function_binding expression={expression:?}");
            }
            self.push_i32_const(JS_TYPEOF_FUNCTION_TAG);
            return Ok(());
        }
        if trace_typeof {
            eprintln!(
                "emit_typeof_expression:function_binding_probe_done expression={expression:?}"
            );
        }
        if let Expression::Identifier(name) = expression
            && self.emit_typeof_implicit_global_binding(name)?
        {
            if trace_typeof {
                eprintln!("emit_typeof_expression:implicit_global name={name}");
            }
            return Ok(());
        }
        if let Expression::Identifier(name) = expression
            && self
                .backend
                .global_property_descriptor(name)
                .or_else(|| {
                    self.backend
                        .shared_global_semantics
                        .values
                        .property_descriptor(name)
                })
                .is_some_and(|state| state.has_get || state.getter.is_some())
        {
            if trace_typeof {
                eprintln!("emit_typeof_expression:global_accessor name={name}");
            }
            self.emit_runtime_typeof_tag(expression)?;
            return Ok(());
        }
        if let Some(strict) = self.resolve_arguments_callee_strictness(expression) {
            if strict {
                if trace_typeof {
                    eprintln!("emit_typeof_expression:arguments_callee_strict");
                }
                return self.emit_error_throw();
            }
            if trace_typeof {
                eprintln!("emit_typeof_expression:arguments_callee_non_strict");
            }
            self.push_i32_const(JS_TYPEOF_FUNCTION_TAG);
            return Ok(());
        }
        if let Expression::Identifier(name) = expression
            && self.is_identifier_bound(name)
        {
            if trace_typeof {
                eprintln!("emit_typeof_expression:bound_identifier name={name}");
            }
            self.emit_runtime_typeof_tag(expression)?;
            return Ok(());
        }
        if trace_typeof {
            eprintln!("emit_typeof_expression:infer_start expression={expression:?}");
        }
        let Some(type_tag) = self
            .infer_typeof_operand_kind(expression)
            .and_then(StaticValueKind::as_typeof_tag)
        else {
            if trace_typeof {
                eprintln!("emit_typeof_expression:runtime_start expression={expression:?}");
            }
            self.emit_runtime_typeof_tag(expression)?;
            if trace_typeof {
                eprintln!("emit_typeof_expression:runtime_done expression={expression:?}");
            }
            return Ok(());
        };
        if trace_typeof {
            eprintln!("emit_typeof_expression:static_tag expression={expression:?} tag={type_tag}");
        }
        self.push_i32_const(type_tag);
        Ok(())
    }
}
