use super::super::*;

fn static_call_arguments_match(lhs: &CallArgument, rhs: &CallArgument) -> bool {
    match (lhs, rhs) {
        (CallArgument::Expression(left), CallArgument::Expression(right))
        | (CallArgument::Spread(left), CallArgument::Spread(right)) => {
            static_expression_matches(left, right)
        }
        _ => false,
    }
}

fn static_array_elements_match(lhs: &ArrayElement, rhs: &ArrayElement) -> bool {
    match (lhs, rhs) {
        (ArrayElement::Expression(left), ArrayElement::Expression(right))
        | (ArrayElement::Spread(left), ArrayElement::Spread(right)) => {
            static_expression_matches(left, right)
        }
        _ => false,
    }
}

fn static_object_entries_match(lhs: &ObjectEntry, rhs: &ObjectEntry) -> bool {
    match (lhs, rhs) {
        (
            ObjectEntry::Data {
                key: left_key,
                value: left_value,
            },
            ObjectEntry::Data {
                key: right_key,
                value: right_value,
            },
        ) => {
            static_expression_matches(left_key, right_key)
                && static_expression_matches(left_value, right_value)
        }
        (
            ObjectEntry::Getter {
                key: left_key,
                getter: left_getter,
            },
            ObjectEntry::Getter {
                key: right_key,
                getter: right_getter,
            },
        ) => {
            static_expression_matches(left_key, right_key)
                && static_expression_matches(left_getter, right_getter)
        }
        (
            ObjectEntry::Setter {
                key: left_key,
                setter: left_setter,
            },
            ObjectEntry::Setter {
                key: right_key,
                setter: right_setter,
            },
        ) => {
            static_expression_matches(left_key, right_key)
                && static_expression_matches(left_setter, right_setter)
        }
        (ObjectEntry::Spread(left), ObjectEntry::Spread(right)) => {
            static_expression_matches(left, right)
        }
        _ => false,
    }
}

fn static_expression_slices_match(lhs: &[Expression], rhs: &[Expression]) -> bool {
    lhs.len() == rhs.len()
        && lhs
            .iter()
            .zip(rhs.iter())
            .all(|(left, right)| static_expression_matches(left, right))
}

fn static_call_argument_slices_match(lhs: &[CallArgument], rhs: &[CallArgument]) -> bool {
    lhs.len() == rhs.len()
        && lhs
            .iter()
            .zip(rhs.iter())
            .all(|(left, right)| static_call_arguments_match(left, right))
}

pub(in crate::backend::direct_wasm) fn static_expression_matches(
    lhs: &Expression,
    rhs: &Expression,
) -> bool {
    if lhs == rhs {
        return true;
    }
    match (lhs, rhs) {
        (Expression::Number(left), Expression::Number(right)) => {
            (left.is_nan() && right.is_nan()) || left == right
        }
        (Expression::Array(left), Expression::Array(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right.iter())
                    .all(|(left, right)| static_array_elements_match(left, right))
        }
        (Expression::Object(left), Expression::Object(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right.iter())
                    .all(|(left, right)| static_object_entries_match(left, right))
        }
        (
            Expression::Member {
                object: left_object,
                property: left_property,
            },
            Expression::Member {
                object: right_object,
                property: right_property,
            },
        ) => {
            static_expression_matches(left_object, right_object)
                && static_expression_matches(left_property, right_property)
        }
        (
            Expression::SuperMember {
                property: left_property,
            },
            Expression::SuperMember {
                property: right_property,
            },
        ) => static_expression_matches(left_property, right_property),
        (
            Expression::Assign {
                name: left_name,
                value: left_value,
            },
            Expression::Assign {
                name: right_name,
                value: right_value,
            },
        ) => left_name == right_name && static_expression_matches(left_value, right_value),
        (
            Expression::AssignMember {
                object: left_object,
                property: left_property,
                value: left_value,
            },
            Expression::AssignMember {
                object: right_object,
                property: right_property,
                value: right_value,
            },
        ) => {
            static_expression_matches(left_object, right_object)
                && static_expression_matches(left_property, right_property)
                && static_expression_matches(left_value, right_value)
        }
        (
            Expression::AssignSuperMember {
                property: left_property,
                value: left_value,
            },
            Expression::AssignSuperMember {
                property: right_property,
                value: right_value,
            },
        ) => {
            static_expression_matches(left_property, right_property)
                && static_expression_matches(left_value, right_value)
        }
        (Expression::Await(left), Expression::Await(right))
        | (Expression::EnumerateKeys(left), Expression::EnumerateKeys(right))
        | (Expression::GetIterator(left), Expression::GetIterator(right))
        | (Expression::IteratorClose(left), Expression::IteratorClose(right)) => {
            static_expression_matches(left, right)
        }
        (
            Expression::Unary {
                op: left_op,
                expression: left_expression,
            },
            Expression::Unary {
                op: right_op,
                expression: right_expression,
            },
        ) => left_op == right_op && static_expression_matches(left_expression, right_expression),
        (
            Expression::Binary {
                op: left_op,
                left: left_left,
                right: left_right,
            },
            Expression::Binary {
                op: right_op,
                left: right_left,
                right: right_right,
            },
        ) => {
            left_op == right_op
                && static_expression_matches(left_left, right_left)
                && static_expression_matches(left_right, right_right)
        }
        (
            Expression::Conditional {
                condition: left_condition,
                then_expression: left_then,
                else_expression: left_else,
            },
            Expression::Conditional {
                condition: right_condition,
                then_expression: right_then,
                else_expression: right_else,
            },
        ) => {
            static_expression_matches(left_condition, right_condition)
                && static_expression_matches(left_then, right_then)
                && static_expression_matches(left_else, right_else)
        }
        (Expression::Sequence(left), Expression::Sequence(right)) => {
            static_expression_slices_match(left, right)
        }
        (
            Expression::Call {
                callee: left_callee,
                arguments: left_arguments,
            },
            Expression::Call {
                callee: right_callee,
                arguments: right_arguments,
            },
        )
        | (
            Expression::SuperCall {
                callee: left_callee,
                arguments: left_arguments,
            },
            Expression::SuperCall {
                callee: right_callee,
                arguments: right_arguments,
            },
        )
        | (
            Expression::New {
                callee: left_callee,
                arguments: left_arguments,
            },
            Expression::New {
                callee: right_callee,
                arguments: right_arguments,
            },
        ) => {
            static_expression_matches(left_callee, right_callee)
                && static_call_argument_slices_match(left_arguments, right_arguments)
        }
        (
            Expression::Update {
                name: left_name,
                op: left_op,
                prefix: left_prefix,
            },
            Expression::Update {
                name: right_name,
                op: right_op,
                prefix: right_prefix,
            },
        ) => left_name == right_name && left_op == right_op && left_prefix == right_prefix,
        _ => false,
    }
}
