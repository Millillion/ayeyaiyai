use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn user_function_rest_parameter_index(
        &self,
        user_function: &UserFunction,
    ) -> Option<usize> {
        self.resolve_registered_function_declaration(&user_function.name)?
            .params
            .iter()
            .position(|parameter| parameter.rest)
    }

    pub(in crate::backend::direct_wasm) fn emit_user_function_runtime_call_from_expanded_arguments(
        &mut self,
        user_function: &UserFunction,
        expanded_arguments: &[Expression],
        updated_bindings: Option<&HashMap<String, Expression>>,
    ) -> DirectResult<(u32, Vec<(String, String, Option<ObjectValueBinding>)>)> {
        let trace_user_calls = std::env::var_os("AYY_TRACE_USER_CALLS").is_some();
        if trace_user_calls {
            eprintln!(
                "runtime_call:start current_fn={:?} target={} args={expanded_arguments:?}",
                self.current_function_name(),
                user_function.name
            );
        }
        let parameter_object_shadow_writebacks = self
            .emit_user_function_parameter_object_shadow_setup(user_function, expanded_arguments)?;
        if trace_user_calls {
            eprintln!(
                "runtime_call:after_shadow_setup target={} writebacks={}",
                user_function.name,
                parameter_object_shadow_writebacks.len()
            );
        }
        let visible_param_count = user_function.visible_param_count() as usize;
        let rest_parameter_index = self.user_function_rest_parameter_index(user_function);
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
                self.emit_numeric_expression(argument)?;
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
                self.emit_numeric_expression(argument)?;
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
            if Some(argument_index) == rest_parameter_index {
                self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            } else if let Some(argument_local) = argument_locals.get(&argument_index).copied() {
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
        self.emit_user_function_parameter_object_shadow_writeback(
            &parameter_object_shadow_writebacks,
        )?;
        self.sync_user_function_parameter_object_shadow_writeback_static_metadata(
            &parameter_object_shadow_writebacks,
            updated_bindings,
        );
        Ok((return_value_local, parameter_object_shadow_writebacks))
    }
}
