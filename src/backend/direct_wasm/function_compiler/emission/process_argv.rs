use super::*;

impl<'a> FunctionCompiler<'a> {
    fn process_argv_length_binding(&mut self) -> ImplicitGlobalBinding {
        self.backend
            .mark_global_array_with_runtime_state(PROCESS_ARGV_GLOBAL_NAME);
        self.backend
            .shared_global_semantics
            .values
            .mark_array_with_runtime_state(PROCESS_ARGV_GLOBAL_NAME);
        self.global_runtime_array_length_binding(PROCESS_ARGV_GLOBAL_NAME)
    }

    fn process_argv_slot_binding(&mut self, index: u32) -> ImplicitGlobalBinding {
        self.backend
            .mark_global_array_with_runtime_state(PROCESS_ARGV_GLOBAL_NAME);
        self.backend
            .shared_global_semantics
            .values
            .mark_array_with_runtime_state(PROCESS_ARGV_GLOBAL_NAME);
        self.global_runtime_array_slot_binding(PROCESS_ARGV_GLOBAL_NAME, index)
    }

    fn emit_process_argv_set_length_from_wasi_argc(&mut self) -> DirectResult<()> {
        let binding = self.process_argv_length_binding();
        self.push_i32_const(PROCESS_ARGV_ARGC_OFFSET as i32);
        self.push_memory_i32_load(0);
        self.push_i32_const(1);
        self.push_binary_op(BinaryOp::Add)?;
        self.push_global_set(binding.value_index);
        self.push_i32_const(1);
        self.push_global_set(binding.present_index);
        Ok(())
    }

    fn emit_process_argv_set_slot_const(&mut self, index: u32, value: i32) {
        let binding = self.process_argv_slot_binding(index);
        self.push_i32_const(value);
        self.push_global_set(binding.value_index);
        self.push_i32_const(1);
        self.push_global_set(binding.present_index);
    }

    fn emit_process_argv_clear_slot(&mut self, index: u32) {
        let binding = self.process_argv_slot_binding(index);
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_global_set(binding.value_index);
        self.push_i32_const(0);
        self.push_global_set(binding.present_index);
    }

    fn initialize_process_argv_object_globals(&mut self) {
        for name in [PROCESS_GLOBAL_NAME, PROCESS_ARGV_GLOBAL_NAME] {
            if let Some(global_index) = self.backend.global_binding_index(name) {
                self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
                self.push_global_set(global_index);
            }
        }
    }

    fn emit_process_argv_args_fit_guard(&mut self) -> DirectResult<()> {
        self.push_i32_const(PROCESS_ARGV_ARGC_OFFSET as i32);
        self.push_memory_i32_load(0);
        self.push_i32_const(PROCESS_ARGV_TRACKED_WASI_ARG_LIMIT as i32);
        self.push_binary_op(BinaryOp::LessThanOrEqual)?;
        self.push_i32_const(PROCESS_ARGV_BUFFER_SIZE_OFFSET as i32);
        self.push_memory_i32_load(0);
        self.push_i32_const(PROCESS_ARGV_RAW_BUFFER_CAPACITY as i32);
        self.push_binary_op(BinaryOp::LessThanOrEqual)?;
        self.push_binary_op(BinaryOp::BitwiseAnd)?;
        Ok(())
    }

    fn emit_process_argv_copy_slot_loop(
        &mut self,
        raw_ptr_local: u32,
        dst_ptr_local: u32,
        len_local: u32,
        byte_local: u32,
    ) -> DirectResult<()> {
        self.state.emission.output.instructions.push(0x02);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        let break_target = self.push_control_frame();

        self.state.emission.output.instructions.push(0x03);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        let loop_target = self.push_control_frame();

        self.push_local_get(len_local);
        self.push_i32_const(PROCESS_ARGV_SLOT_STRING_CAPACITY as i32);
        self.push_binary_op(BinaryOp::GreaterThanOrEqual)?;
        self.push_br_if(self.relative_depth(break_target));

        self.push_local_get(raw_ptr_local);
        self.push_memory_i32_load8_u(0);
        self.push_local_set(byte_local);
        self.push_local_get(byte_local);
        self.state.emission.output.instructions.push(0x45);
        self.push_br_if(self.relative_depth(break_target));

        self.push_local_get(dst_ptr_local);
        self.push_local_get(byte_local);
        self.push_memory_i32_store8(0);

        self.push_local_get(raw_ptr_local);
        self.push_i32_const(1);
        self.push_binary_op(BinaryOp::Add)?;
        self.push_local_set(raw_ptr_local);

        self.push_local_get(dst_ptr_local);
        self.push_i32_const(1);
        self.push_binary_op(BinaryOp::Add)?;
        self.push_local_set(dst_ptr_local);

        self.push_local_get(len_local);
        self.push_i32_const(1);
        self.push_binary_op(BinaryOp::Add)?;
        self.push_local_set(len_local);

        self.push_br(self.relative_depth(loop_target));
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }

    fn emit_process_argv_copy_wasi_arg(
        &mut self,
        layout: ProcessArgvRuntimeLayout,
        wasi_index: u32,
        raw_ptr_local: u32,
        dst_ptr_local: u32,
        len_local: u32,
        byte_local: u32,
    ) -> DirectResult<()> {
        self.push_i32_const(PROCESS_ARGV_ARGC_OFFSET as i32);
        self.push_memory_i32_load(0);
        self.push_i32_const(wasi_index as i32);
        self.push_binary_op(BinaryOp::GreaterThan)?;
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();

        self.push_i32_const((layout.argv_pointers_offset + wasi_index * 4) as i32);
        self.push_memory_i32_load(0);
        self.push_local_set(raw_ptr_local);

        let string_base = layout.string_slots_offset + wasi_index * PROCESS_ARGV_SLOT_STRIDE;
        let string_ptr = string_base + STRING_LENGTH_PREFIX_SIZE;
        self.push_i32_const(string_ptr as i32);
        self.push_local_set(dst_ptr_local);
        self.push_i32_const(0);
        self.push_local_set(len_local);

        self.emit_process_argv_copy_slot_loop(raw_ptr_local, dst_ptr_local, len_local, byte_local)?;

        self.push_i32_const(string_base as i32);
        self.push_local_get(len_local);
        self.push_memory_i32_store(0);
        self.emit_process_argv_set_slot_const(wasi_index + 1, string_ptr as i32);

        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn initialize_process_argv_runtime_array(
        &mut self,
    ) -> DirectResult<()> {
        let Some(layout) = self.backend.process_argv_layout else {
            return Ok(());
        };
        if self.current_function_name().is_some() {
            return Ok(());
        }

        self.initialize_process_argv_object_globals();
        self.emit_process_argv_set_slot_const(0, layout.display_name_ptr as i32);
        for index in 1..TRACKED_ARRAY_SLOT_LIMIT {
            self.emit_process_argv_clear_slot(index);
        }

        self.push_i32_const(PROCESS_ARGV_ARGC_OFFSET as i32);
        self.push_i32_const(PROCESS_ARGV_BUFFER_SIZE_OFFSET as i32);
        self.push_call(ARGS_SIZES_GET_FUNCTION_INDEX);
        self.state.emission.output.instructions.push(0x1a);
        self.emit_process_argv_set_length_from_wasi_argc()?;

        self.emit_process_argv_args_fit_guard()?;
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();

        self.push_i32_const(layout.argv_pointers_offset as i32);
        self.push_i32_const(layout.raw_buffer_offset as i32);
        self.push_call(ARGS_GET_FUNCTION_INDEX);
        self.state.emission.output.instructions.push(0x1a);

        let raw_ptr_local = self.allocate_temp_local();
        let dst_ptr_local = self.allocate_temp_local();
        let len_local = self.allocate_temp_local();
        let byte_local = self.allocate_temp_local();

        for wasi_index in 0..PROCESS_ARGV_TRACKED_WASI_ARG_LIMIT {
            self.emit_process_argv_copy_wasi_arg(
                layout,
                wasi_index,
                raw_ptr_local,
                dst_ptr_local,
                len_local,
                byte_local,
            )?;
        }

        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }
}
