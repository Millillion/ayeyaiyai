use super::*;

impl<'a> FunctionCompiler<'a> {
    fn emit_static_runtime_call_argument_if_resolved(
        &mut self,
        argument: &Expression,
    ) -> DirectResult<bool> {
        if Self::expression_contains_assignment_or_update(argument)
            || Self::expression_contains_call_or_construct(argument)
        {
            return Ok(false);
        }
        if let Some(value) = self.resolve_fast_static_boolean_expression(argument, 0) {
            self.emit_literal_expression(&Expression::Bool(value))?;
            return Ok(true);
        }
        if let Some(value) = self.resolve_fast_static_number_expression(argument, 0) {
            self.emit_literal_expression(&Expression::Number(value))?;
            return Ok(true);
        }
        Ok(false)
    }

    fn emit_runtime_call_argument_expression(&mut self, argument: &Expression) -> DirectResult<()> {
        if self.emit_static_runtime_call_argument_if_resolved(argument)? {
            return Ok(());
        }
        self.emit_numeric_expression(argument)
    }

    pub(in crate::backend::direct_wasm) fn emit_user_function_runtime_call_from_expanded_arguments(
        &mut self,
        user_function: &UserFunction,
        expanded_arguments: &[Expression],
        updated_bindings: Option<&HashMap<String, Expression>>,
        skip_static_argument_member_writebacks: bool,
    ) -> DirectResult<(
        u32,
        Vec<(String, String, Option<ObjectValueBinding>)>,
        Vec<(String, String, BTreeMap<String, Expression>)>,
    )> {
        let trace_user_calls = crate::ayy_env_flag!("AYY_TRACE_USER_CALLS");
        if trace_user_calls {
            eprintln!(
                "runtime_call:start current_fn={:?} target={} args={expanded_arguments:?}",
                self.current_function_name(),
                user_function.name
            );
        }
        let module_init_call = user_function.name.starts_with("__ayy_module_init_");
        let static_argument_member_writebacks =
            if module_init_call || skip_static_argument_member_writebacks {
                Vec::new()
            } else {
                self.user_function_static_argument_object_member_writeback_values(
                    user_function,
                    expanded_arguments,
                )
            };
        if trace_user_calls {
            eprintln!(
                "runtime_call:after_static_arg_writebacks target={} count={}",
                user_function.name,
                static_argument_member_writebacks.len()
            );
        }
        self.predeclare_static_argument_object_member_writeback_properties(
            &static_argument_member_writebacks,
        );

        let parameter_object_shadow_writebacks = if module_init_call {
            Vec::new()
        } else {
            self.emit_user_function_parameter_object_shadow_setup(
                user_function,
                expanded_arguments,
            )?
        };
        if trace_user_calls {
            eprintln!(
                "runtime_call:after_shadow_setup target={} writebacks={}",
                user_function.name,
                parameter_object_shadow_writebacks.len()
            );
        }
        let visible_param_count = user_function.visible_param_count() as usize;
        let tracked_extra_indices = user_function
            .extra_argument_indices
            .iter()
            .map(|index| *index as usize)
            .collect::<HashSet<_>>();
        let mut argument_locals = HashMap::new();

        for (argument_index, argument) in expanded_arguments.iter().enumerate() {
            if argument_index < visible_param_count
                || tracked_extra_indices.contains(&argument_index)
            {
                let argument_local = self.allocate_temp_local();
                if trace_user_calls {
                    eprintln!(
                        "runtime_call:emit_arg_start target={} index={} local={} arg={argument:?}",
                        user_function.name, argument_index, argument_local
                    );
                }
                self.emit_runtime_call_argument_expression(argument)?;
                if trace_user_calls {
                    eprintln!(
                        "runtime_call:emit_arg_done target={} index={}",
                        user_function.name, argument_index
                    );
                }
                self.push_local_set(argument_local);
                argument_locals.insert(argument_index, argument_local);
            } else {
                if trace_user_calls {
                    eprintln!(
                        "runtime_call:emit_ignored_arg_start target={} index={} arg={argument:?}",
                        user_function.name, argument_index
                    );
                }
                self.emit_runtime_call_argument_expression(argument)?;
                if trace_user_calls {
                    eprintln!(
                        "runtime_call:emit_ignored_arg_done target={} index={}",
                        user_function.name, argument_index
                    );
                }
                self.state.emission.output.instructions.push(0x1a);
            }
        }
        if trace_user_calls {
            eprintln!("runtime_call:after_emit_args target={}", user_function.name);
        }

        for argument_index in 0..visible_param_count {
            if let Some(argument_local) = argument_locals.get(&argument_index).copied() {
                self.push_local_get(argument_local);
            } else {
                self.push_i32_const(JS_UNDEFINED_TAG);
            }
        }

        self.push_i32_const(expanded_arguments.len() as i32);

        for index in &user_function.extra_argument_indices {
            if let Some(argument_local) = argument_locals.get(&(*index as usize)).copied() {
                self.push_local_get(argument_local);
            } else {
                self.push_i32_const(JS_UNDEFINED_TAG);
            }
        }
        self.push_user_function_call(user_function);
        let return_value_local = self.allocate_temp_local();
        self.push_local_set(return_value_local);
        if trace_user_calls {
            eprintln!(
                "runtime_call:after_call target={} return_local={}",
                user_function.name, return_value_local
            );
        }
        if !module_init_call {
            self.emit_user_function_parameter_object_shadow_writeback(
                &parameter_object_shadow_writebacks,
            )?;
            self.sync_user_function_parameter_object_shadow_writeback_static_metadata(
                &parameter_object_shadow_writebacks,
                updated_bindings,
            );
        }
        self.sync_static_argument_object_member_writeback_values(
            &static_argument_member_writebacks,
        );
        Ok((
            return_value_local,
            parameter_object_shadow_writebacks,
            static_argument_member_writebacks,
        ))
    }
}
