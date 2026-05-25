use super::*;

impl DirectWasmCompiler {
    pub(in crate::backend::direct_wasm) fn collect_parameter_value_bindings_from_expression(
        &self,
        expression: &Expression,
        aliases: &mut HashMap<String, Option<LocalFunctionBinding>>,
        bindings: &mut HashMap<String, HashMap<String, Option<Expression>>>,
    ) {
        let source_bindings = bindings.clone();
        self.collect_parameter_value_bindings_from_expression_in_function(
            expression,
            aliases,
            bindings,
            &source_bindings,
            None,
        );
    }

    pub(in crate::backend::direct_wasm) fn collect_parameter_value_bindings_from_expression_in_function(
        &self,
        expression: &Expression,
        aliases: &mut HashMap<String, Option<LocalFunctionBinding>>,
        bindings: &mut HashMap<String, HashMap<String, Option<Expression>>>,
        source_bindings: &HashMap<String, HashMap<String, Option<Expression>>>,
        current_function_name: Option<&str>,
    ) {
        match expression {
            Expression::Call { callee, arguments } => {
                self.collect_parameter_value_bindings_from_expression_in_function(
                    callee,
                    aliases,
                    bindings,
                    source_bindings,
                    current_function_name,
                );
                self.register_parameter_value_bindings_for_call(
                    callee,
                    arguments,
                    aliases,
                    bindings,
                    source_bindings,
                    current_function_name,
                );
                for argument in arguments {
                    let argument = match argument {
                        CallArgument::Expression(argument) | CallArgument::Spread(argument) => {
                            argument
                        }
                    };
                    self.collect_parameter_value_bindings_from_expression_in_function(
                        argument,
                        aliases,
                        bindings,
                        source_bindings,
                        current_function_name,
                    );
                }
            }
            Expression::Assign { name, value } => {
                self.collect_parameter_value_bindings_from_expression_in_function(
                    value,
                    aliases,
                    bindings,
                    source_bindings,
                    current_function_name,
                );
                aliases.insert(
                    name.clone(),
                    self.resolve_function_binding_from_expression_with_aliases(value, aliases),
                );
            }
            Expression::Member { object, property } => {
                self.collect_parameter_value_bindings_from_expression_in_function(
                    object,
                    aliases,
                    bindings,
                    source_bindings,
                    current_function_name,
                );
                self.collect_parameter_value_bindings_from_expression_in_function(
                    property,
                    aliases,
                    bindings,
                    source_bindings,
                    current_function_name,
                );
            }
            Expression::SuperMember { property } => {
                self.collect_parameter_value_bindings_from_expression_in_function(
                    property,
                    aliases,
                    bindings,
                    source_bindings,
                    current_function_name,
                );
            }
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.collect_parameter_value_bindings_from_expression_in_function(
                    object,
                    aliases,
                    bindings,
                    source_bindings,
                    current_function_name,
                );
                self.collect_parameter_value_bindings_from_expression_in_function(
                    property,
                    aliases,
                    bindings,
                    source_bindings,
                    current_function_name,
                );
                self.collect_parameter_value_bindings_from_expression_in_function(
                    value,
                    aliases,
                    bindings,
                    source_bindings,
                    current_function_name,
                );
            }
            Expression::AssignSuperMember { property, value } => {
                self.collect_parameter_value_bindings_from_expression_in_function(
                    property,
                    aliases,
                    bindings,
                    source_bindings,
                    current_function_name,
                );
                self.collect_parameter_value_bindings_from_expression_in_function(
                    value,
                    aliases,
                    bindings,
                    source_bindings,
                    current_function_name,
                );
            }
            Expression::Unary { expression, .. }
            | Expression::Await(expression)
            | Expression::EnumerateKeys(expression)
            | Expression::GetIterator(expression)
            | Expression::IteratorClose(expression) => {
                self.collect_parameter_value_bindings_from_expression_in_function(
                    expression,
                    aliases,
                    bindings,
                    source_bindings,
                    current_function_name,
                );
            }
            Expression::Array(elements) => {
                for element in elements {
                    let expression = match element {
                        ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                            expression
                        }
                    };
                    self.collect_parameter_value_bindings_from_expression_in_function(
                        expression,
                        aliases,
                        bindings,
                        source_bindings,
                        current_function_name,
                    );
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            self.collect_parameter_value_bindings_from_expression_in_function(
                                key,
                                aliases,
                                bindings,
                                source_bindings,
                                current_function_name,
                            );
                            self.collect_parameter_value_bindings_from_expression_in_function(
                                value,
                                aliases,
                                bindings,
                                source_bindings,
                                current_function_name,
                            );
                        }
                        ObjectEntry::Getter { key, getter } => {
                            self.collect_parameter_value_bindings_from_expression_in_function(
                                key,
                                aliases,
                                bindings,
                                source_bindings,
                                current_function_name,
                            );
                            self.collect_parameter_value_bindings_from_expression_in_function(
                                getter,
                                aliases,
                                bindings,
                                source_bindings,
                                current_function_name,
                            );
                        }
                        ObjectEntry::Setter { key, setter } => {
                            self.collect_parameter_value_bindings_from_expression_in_function(
                                key,
                                aliases,
                                bindings,
                                source_bindings,
                                current_function_name,
                            );
                            self.collect_parameter_value_bindings_from_expression_in_function(
                                setter,
                                aliases,
                                bindings,
                                source_bindings,
                                current_function_name,
                            );
                        }
                        ObjectEntry::Spread(expression) => {
                            self.collect_parameter_value_bindings_from_expression_in_function(
                                expression,
                                aliases,
                                bindings,
                                source_bindings,
                                current_function_name,
                            );
                        }
                    }
                }
            }
            Expression::Binary { left, right, .. } => {
                self.collect_parameter_value_bindings_from_expression_in_function(
                    left,
                    aliases,
                    bindings,
                    source_bindings,
                    current_function_name,
                );
                self.collect_parameter_value_bindings_from_expression_in_function(
                    right,
                    aliases,
                    bindings,
                    source_bindings,
                    current_function_name,
                );
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.collect_parameter_value_bindings_from_expression_in_function(
                    condition,
                    aliases,
                    bindings,
                    source_bindings,
                    current_function_name,
                );
                self.collect_parameter_value_bindings_from_expression_in_function(
                    then_expression,
                    aliases,
                    bindings,
                    source_bindings,
                    current_function_name,
                );
                self.collect_parameter_value_bindings_from_expression_in_function(
                    else_expression,
                    aliases,
                    bindings,
                    source_bindings,
                    current_function_name,
                );
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    self.collect_parameter_value_bindings_from_expression_in_function(
                        expression,
                        aliases,
                        bindings,
                        source_bindings,
                        current_function_name,
                    );
                }
            }
            Expression::New { callee, arguments } | Expression::SuperCall { callee, arguments } => {
                self.collect_parameter_value_bindings_from_expression_in_function(
                    callee,
                    aliases,
                    bindings,
                    source_bindings,
                    current_function_name,
                );
                self.register_constructor_parameter_value_bindings_for_call(
                    callee,
                    arguments,
                    aliases,
                    bindings,
                    source_bindings,
                    current_function_name,
                );
                for argument in arguments {
                    let argument = match argument {
                        CallArgument::Expression(argument) | CallArgument::Spread(argument) => {
                            argument
                        }
                    };
                    self.collect_parameter_value_bindings_from_expression_in_function(
                        argument,
                        aliases,
                        bindings,
                        source_bindings,
                        current_function_name,
                    );
                }
            }
            Expression::Update { .. }
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::Identifier(_)
            | Expression::This
            | Expression::Sent
            | Expression::NewTarget => {}
        }
    }
}
