use super::*;

pub(in crate::backend::direct_wasm) fn collect_arguments_usage_from_statements_and_expressions<
    'a,
    I,
>(
    statements: &[Statement],
    expressions: I,
) -> ArgumentsUsage
where
    I: IntoIterator<Item = &'a Expression>,
{
    let mut indexed_slots = HashSet::new();
    let mut track_all_slots = false;
    for statement in statements {
        collect_arguments_usage_from_statement(statement, &mut indexed_slots, &mut track_all_slots);
    }
    for expression in expressions {
        collect_arguments_usage_from_expression(
            expression,
            &mut indexed_slots,
            &mut track_all_slots,
        );
    }
    if track_all_slots {
        indexed_slots.extend(0..TRACKED_ARGUMENT_SLOT_LIMIT);
    }
    let mut indexed_slots = indexed_slots.into_iter().collect::<Vec<_>>();
    indexed_slots.sort_unstable();
    ArgumentsUsage { indexed_slots }
}
