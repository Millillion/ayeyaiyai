use super::super::super::*;

impl DirectWasmCompiler {
    pub(in crate::backend::direct_wasm) fn handle_call_parameter_expression(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
        aliases: &mut HashMap<String, Option<LocalFunctionBinding>>,
        bindings: &mut HashMap<String, HashMap<String, Option<LocalFunctionBinding>>>,
        array_bindings: &mut HashMap<String, HashMap<String, Option<ArrayValueBinding>>>,
        object_bindings: &mut HashMap<String, HashMap<String, Option<ObjectValueBinding>>>,
        current_function_name: Option<&str>,
    ) {
        self.collect_parameter_bindings_from_expression_in_function(
            callee,
            aliases,
            bindings,
            array_bindings,
            object_bindings,
            current_function_name,
        );
        self.register_callback_bindings_for_call(
            callee,
            arguments,
            aliases,
            bindings,
            array_bindings,
            object_bindings,
            current_function_name,
        );
        self.collect_parameter_bindings_from_call_arguments(
            arguments,
            aliases,
            bindings,
            array_bindings,
            object_bindings,
            current_function_name,
        );
    }

    pub(in crate::backend::direct_wasm) fn handle_construct_parameter_expression(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
        aliases: &mut HashMap<String, Option<LocalFunctionBinding>>,
        bindings: &mut HashMap<String, HashMap<String, Option<LocalFunctionBinding>>>,
        array_bindings: &mut HashMap<String, HashMap<String, Option<ArrayValueBinding>>>,
        object_bindings: &mut HashMap<String, HashMap<String, Option<ObjectValueBinding>>>,
        current_function_name: Option<&str>,
    ) {
        self.collect_parameter_bindings_from_expression_in_function(
            callee,
            aliases,
            bindings,
            array_bindings,
            object_bindings,
            current_function_name,
        );
        self.register_constructor_bindings_for_call(
            callee,
            arguments,
            aliases,
            bindings,
            array_bindings,
            object_bindings,
            current_function_name,
        );
        self.collect_parameter_bindings_from_call_arguments(
            arguments,
            aliases,
            bindings,
            array_bindings,
            object_bindings,
            current_function_name,
        );
    }

    fn collect_parameter_bindings_from_call_arguments(
        &self,
        arguments: &[CallArgument],
        aliases: &mut HashMap<String, Option<LocalFunctionBinding>>,
        bindings: &mut HashMap<String, HashMap<String, Option<LocalFunctionBinding>>>,
        array_bindings: &mut HashMap<String, HashMap<String, Option<ArrayValueBinding>>>,
        object_bindings: &mut HashMap<String, HashMap<String, Option<ObjectValueBinding>>>,
        current_function_name: Option<&str>,
    ) {
        for argument in arguments {
            match argument {
                CallArgument::Expression(argument) | CallArgument::Spread(argument) => {
                    self.collect_parameter_bindings_from_expression_in_function(
                        argument,
                        aliases,
                        bindings,
                        array_bindings,
                        object_bindings,
                        current_function_name,
                    );
                    self.collect_parameter_bindings_from_callback_argument(
                        argument,
                        aliases,
                        bindings,
                        array_bindings,
                        object_bindings,
                        current_function_name,
                    );
                }
            }
        }
    }

    fn collect_parameter_bindings_from_callback_argument(
        &self,
        argument: &Expression,
        aliases: &HashMap<String, Option<LocalFunctionBinding>>,
        bindings: &mut HashMap<String, HashMap<String, Option<LocalFunctionBinding>>>,
        array_bindings: &mut HashMap<String, HashMap<String, Option<ArrayValueBinding>>>,
        object_bindings: &mut HashMap<String, HashMap<String, Option<ObjectValueBinding>>>,
        current_function_name: Option<&str>,
    ) {
        let Some(LocalFunctionBinding::User(callback_name)) =
            self.resolve_function_binding_from_expression_with_aliases(argument, aliases)
        else {
            return;
        };
        if current_function_name == Some(callback_name.as_str()) {
            return;
        }
        let Some(callback_function) = self.registered_function(&callback_name) else {
            return;
        };
        let mut callback_aliases = aliases.clone();
        for parameter in &callback_function.params {
            callback_aliases
                .entry(parameter.name.clone())
                .or_insert(None);
        }
        self.collect_parameter_bindings_from_statements_in_function(
            &callback_function.body,
            &mut callback_aliases,
            bindings,
            array_bindings,
            object_bindings,
            Some(&callback_name),
        );
    }
}
