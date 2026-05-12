use super::*;

impl<'a> FunctionCompiler<'a> {
    fn array_binding_matches_numeric_modulo_sequence(
        &self,
        actual: &ArrayValueBinding,
        expected: &ArrayValueBinding,
    ) -> bool {
        let [
            Some(Expression::Binary {
                op: BinaryOp::Modulo,
                left,
                right,
            }),
        ] = expected.values.as_slice()
        else {
            return false;
        };
        let Expression::Identifier(_) = left.as_ref() else {
            return false;
        };
        let Some(modulo) = self.resolve_static_number_value(right) else {
            return false;
        };
        if !modulo.is_finite() || modulo <= 0.0 || modulo.fract() != 0.0 {
            return false;
        }
        let Some(start) = actual.values.first().and_then(|value| {
            value
                .as_ref()
                .and_then(|value| self.resolve_static_number_value(value))
        }) else {
            return actual.values.is_empty();
        };
        if !start.is_finite() || start.fract() != 0.0 || start < 0.0 {
            return false;
        }
        actual.values.iter().enumerate().all(|(offset, value)| {
            let Some(number) = value
                .as_ref()
                .and_then(|value| self.resolve_static_number_value(value))
            else {
                return false;
            };
            number == (start + offset as f64) % modulo
        })
    }

    pub(in crate::backend::direct_wasm) fn static_expressions_equal(
        &self,
        actual: &Expression,
        expected: &Expression,
    ) -> bool {
        if let (Some(actual_text), Some(expected_text)) = (
            self.resolve_static_string_value(actual),
            self.resolve_static_string_value(expected),
        ) {
            return actual_text == expected_text;
        }

        if let (Some(actual_number), Some(expected_number)) = (
            self.resolve_static_number_value(actual),
            self.resolve_static_number_value(expected),
        ) {
            return actual_number == expected_number;
        }

        self.materialize_static_expression(actual) == self.materialize_static_expression(expected)
    }

    pub(in crate::backend::direct_wasm) fn array_bindings_equal(
        &self,
        actual: &ArrayValueBinding,
        expected: &ArrayValueBinding,
    ) -> bool {
        if self.array_binding_matches_numeric_modulo_sequence(actual, expected) {
            return true;
        }
        actual.values.len() == expected.values.len()
            && actual.values.iter().zip(expected.values.iter()).all(
                |(actual_value, expected_value)| match (actual_value, expected_value) {
                    (None, None) => true,
                    (Some(actual_value), Some(expected_value)) => {
                        self.static_expressions_equal(actual_value, expected_value)
                    }
                    _ => false,
                },
            )
    }
}
