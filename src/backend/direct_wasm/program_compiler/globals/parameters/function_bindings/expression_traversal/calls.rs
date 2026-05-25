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
                }
            }
        }
    }
}
