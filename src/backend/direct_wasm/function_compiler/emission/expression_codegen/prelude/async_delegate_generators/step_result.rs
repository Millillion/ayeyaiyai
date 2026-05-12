use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(super) fn emit_async_yield_delegate_step_result_getter_assignment(
        &mut self,
        step_result_name: &str,
        runtime_step_result_expression: &Expression,
        destination_name: &str,
        property_name: &str,
        snapshot_bindings: Option<&mut HashMap<String, Expression>>,
        current_function_name: Option<&str>,
    ) -> DirectResult<bool> {
        let property_expression = Expression::String(property_name.to_string());
        let static_step_result_expression = Expression::Identifier(step_result_name.to_string());
        let Some(LocalFunctionBinding::User(getter_name)) = self
            .resolve_member_getter_binding(&static_step_result_expression, &property_expression)
        else {
            return Ok(false);
        };
        let Some(getter_user_function) = self.user_function(&getter_name).cloned() else {
            return Ok(false);
        };
        let capture_slots = self.resolve_member_function_capture_slots(
            &static_step_result_expression,
            &property_expression,
        );
        if let Some(capture_slots) = capture_slots.as_ref() {
            if snapshot_bindings.is_some() {
                self.emit_user_function_call_with_new_target_and_this_expression_and_bound_captures_without_static_snapshot(
                    &getter_user_function,
                    &[],
                    JS_UNDEFINED_TAG,
                    runtime_step_result_expression,
                    capture_slots,
                )?;
            } else {
                self.emit_user_function_call_with_new_target_and_this_expression_and_bound_captures(
                    &getter_user_function,
                    &[],
                    JS_UNDEFINED_TAG,
                    runtime_step_result_expression,
                    capture_slots,
                )?;
            }
        } else {
            if snapshot_bindings.is_some() {
                self.emit_user_function_call_with_new_target_and_this_expression_without_static_snapshot(
                    &getter_user_function,
                    &[],
                    JS_UNDEFINED_TAG,
                    runtime_step_result_expression,
                )?;
            } else {
                self.emit_user_function_call_with_new_target_and_this_expression(
                    &getter_user_function,
                    &[],
                    JS_UNDEFINED_TAG,
                    runtime_step_result_expression,
                )?;
            }
        }
        let getter_result_local = self.allocate_temp_local();
        self.push_local_set(getter_result_local);
        let getter_member_expression = Expression::Member {
            object: Box::new(static_step_result_expression.clone()),
            property: Box::new(property_expression.clone()),
        };
        self.emit_store_identifier_value_local(
            destination_name,
            &getter_member_expression,
            getter_result_local,
        )?;
        let getter_result_expression = if let Some(snapshot_bindings) = snapshot_bindings {
            self.evaluate_bound_snapshot_expression(
                &getter_member_expression,
                snapshot_bindings,
                current_function_name,
            )
        } else {
            let getter_binding = LocalFunctionBinding::User(getter_name.clone());
            self.resolve_function_binding_static_return_expression_with_call_frame(
                &getter_binding,
                &[],
                runtime_step_result_expression,
            )
        };
        if let Some(getter_result_expression) = getter_result_expression {
            self.update_local_value_binding(destination_name, &getter_result_expression);
            self.update_local_function_binding(destination_name, &getter_result_expression);
            self.update_local_object_binding(destination_name, &getter_result_expression);
            self.update_object_literal_member_bindings_for_value(
                destination_name,
                &getter_result_expression,
            );
        }
        Ok(true)
    }
}
