use super::*;

impl<'a> FunctionCompiler<'a> {
    fn preserve_bound_snapshot_reference_identity_expression(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        matches!(expression, Expression::Identifier(_))
            .then(|| self.resolve_static_reference_identity_key(expression))
            .flatten()
            .map(|_| expression.clone())
    }

    pub(super) fn evaluate_bound_snapshot_array_literal(
        &self,
        elements: &[ArrayElement],
        bindings: &mut HashMap<String, Expression>,
        current_function_name: Option<&str>,
    ) -> Option<Expression> {
        let mut evaluated_elements = Vec::new();
        for element in elements {
            match element {
                ArrayElement::Expression(expression) => {
                    let value = self
                        .preserve_bound_snapshot_reference_identity_expression(expression)
                        .or_else(|| {
                            self.evaluate_bound_snapshot_expression(
                                expression,
                                bindings,
                                current_function_name,
                            )
                        })?;
                    evaluated_elements.push(ArrayElement::Expression(value));
                }
                ArrayElement::Spread(expression) => {
                    let value = self.evaluate_bound_snapshot_expression(
                        expression,
                        bindings,
                        current_function_name,
                    )?;
                    let spread_elements = self.bound_snapshot_array_expression(&value, bindings)?;
                    for spread_element in spread_elements {
                        let ArrayElement::Expression(value) = spread_element else {
                            return None;
                        };
                        let value = self
                            .preserve_bound_snapshot_reference_identity_expression(&value)
                            .unwrap_or(value);
                        evaluated_elements.push(ArrayElement::Expression(value));
                    }
                }
            }
        }
        Some(Expression::Array(evaluated_elements))
    }

    pub(super) fn evaluate_bound_snapshot_object_literal(
        &self,
        entries: &[ObjectEntry],
        bindings: &mut HashMap<String, Expression>,
        current_function_name: Option<&str>,
    ) -> Option<Expression> {
        Some(Expression::Object(
            entries
                .iter()
                .map(|entry| match entry {
                    ObjectEntry::Data { key, value } => Some(ObjectEntry::Data {
                        key: self.resolve_property_key_expression(key).or_else(|| {
                            self.evaluate_bound_snapshot_expression(
                                key,
                                bindings,
                                current_function_name,
                            )
                        })?,
                        value: self
                            .preserve_bound_snapshot_reference_identity_expression(value)
                            .or_else(|| {
                                self.evaluate_bound_snapshot_expression(
                                    value,
                                    bindings,
                                    current_function_name,
                                )
                            })?,
                    }),
                    ObjectEntry::Getter { key, getter } => Some(ObjectEntry::Getter {
                        key: self.resolve_property_key_expression(key).or_else(|| {
                            self.evaluate_bound_snapshot_expression(
                                key,
                                bindings,
                                current_function_name,
                            )
                        })?,
                        getter: self
                            .preserve_bound_snapshot_reference_identity_expression(getter)
                            .or_else(|| {
                                self.evaluate_bound_snapshot_expression(
                                    getter,
                                    bindings,
                                    current_function_name,
                                )
                            })?,
                    }),
                    ObjectEntry::Setter { key, setter } => Some(ObjectEntry::Setter {
                        key: self.resolve_property_key_expression(key).or_else(|| {
                            self.evaluate_bound_snapshot_expression(
                                key,
                                bindings,
                                current_function_name,
                            )
                        })?,
                        setter: self
                            .preserve_bound_snapshot_reference_identity_expression(setter)
                            .or_else(|| {
                                self.evaluate_bound_snapshot_expression(
                                    setter,
                                    bindings,
                                    current_function_name,
                                )
                            })?,
                    }),
                    ObjectEntry::Spread(_) => None,
                })
                .collect::<Option<Vec<_>>>()?,
        ))
    }
}
