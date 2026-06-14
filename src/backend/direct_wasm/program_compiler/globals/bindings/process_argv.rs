use super::*;

impl DirectWasmCompiler {
    pub(in crate::backend::direct_wasm) fn seed_process_argv_bindings(&mut self) {
        let mut next_global_index = self.next_allocated_global_index();
        self.ensure_global_binding_index(PROCESS_GLOBAL_NAME, &mut next_global_index);
        self.set_global_binding_kind(PROCESS_GLOBAL_NAME, StaticValueKind::Object);
        self.upsert_global_data_property_descriptor(
            PROCESS_GLOBAL_NAME,
            Expression::Identifier(PROCESS_GLOBAL_NAME.to_string()),
            Some(false),
            false,
            true,
        );

        let mut process_binding = self
            .global_object_binding(PROCESS_GLOBAL_NAME)
            .cloned()
            .unwrap_or_else(empty_object_value_binding);
        object_binding_define_property(
            &mut process_binding,
            Expression::String("argv".to_string()),
            Expression::Identifier(PROCESS_ARGV_GLOBAL_NAME.to_string()),
            false,
        );
        self.sync_global_object_binding(PROCESS_GLOBAL_NAME, Some(process_binding));

        let argv_values = (0..TRACKED_ARRAY_SLOT_LIMIT)
            .map(|_| Some(Expression::String(String::new())))
            .collect::<Vec<_>>();
        self.sync_global_array_binding(
            PROCESS_ARGV_GLOBAL_NAME,
            Some(ArrayValueBinding {
                values: argv_values,
            }),
        );
        self.ensure_global_binding_index(PROCESS_ARGV_GLOBAL_NAME, &mut next_global_index);
        self.set_global_binding_kind(PROCESS_ARGV_GLOBAL_NAME, StaticValueKind::Object);
        self.mark_global_array_with_runtime_state(PROCESS_ARGV_GLOBAL_NAME);

        let (display_name_ptr, _) =
            self.intern_string(PROCESS_ARGV_DISPLAY_NAME.as_bytes().to_vec());
        let pointers_len = PROCESS_ARGV_TRACKED_WASI_ARG_LIMIT * 4;
        let string_slots_len = PROCESS_ARGV_TRACKED_WASI_ARG_LIMIT * PROCESS_ARGV_SLOT_STRIDE;
        let reserved_len = pointers_len + PROCESS_ARGV_RAW_BUFFER_CAPACITY + string_slots_len;
        let argv_pointers_offset = self.state.reserve_zeroed_data(reserved_len);
        self.state.process_argv_layout = Some(ProcessArgvRuntimeLayout {
            argv_pointers_offset,
            raw_buffer_offset: argv_pointers_offset + pointers_len,
            string_slots_offset: argv_pointers_offset
                + pointers_len
                + PROCESS_ARGV_RAW_BUFFER_CAPACITY,
            display_name_ptr,
        });
    }
}
