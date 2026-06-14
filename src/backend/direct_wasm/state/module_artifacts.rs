use super::*;

#[derive(Default)]
pub(in crate::backend::direct_wasm) struct ModuleArtifactsState {
    pub(in crate::backend::direct_wasm) string_data: Vec<(u32, Vec<u8>)>,
    pub(in crate::backend::direct_wasm) interned_strings: HashMap<Vec<u8>, (u32, u32)>,
    pub(in crate::backend::direct_wasm) next_data_offset: u32,
}

impl ModuleArtifactsState {
    pub(in crate::backend::direct_wasm) fn reset_for_program(&mut self) {
        self.string_data.clear();
        self.interned_strings.clear();
        self.next_data_offset = DATA_START_OFFSET;
    }

    pub(in crate::backend::direct_wasm) fn intern_string(&mut self, bytes: Vec<u8>) -> (u32, u32) {
        if let Some(existing) = self.interned_strings.get(&bytes) {
            return *existing;
        }

        let offset = self.next_data_offset;
        let len = bytes.len() as u32;
        let ptr = offset + STRING_LENGTH_PREFIX_SIZE;
        let mut data = Vec::with_capacity(STRING_LENGTH_PREFIX_SIZE as usize + bytes.len());
        data.extend_from_slice(&len.to_le_bytes());
        data.extend_from_slice(&bytes);
        self.next_data_offset += STRING_LENGTH_PREFIX_SIZE + len;
        self.string_data.push((offset, data));
        self.interned_strings.insert(bytes, (ptr, len));
        (ptr, len)
    }

    pub(in crate::backend::direct_wasm) fn reserve_zeroed_data(&mut self, len: u32) -> u32 {
        const ALIGNMENT: u32 = 4;
        let padding = (ALIGNMENT - (self.next_data_offset % ALIGNMENT)) % ALIGNMENT;
        self.next_data_offset += padding;
        let offset = self.next_data_offset;
        self.next_data_offset += len;
        offset
    }

    pub(in crate::backend::direct_wasm) fn snapshot_data(&self) -> (Vec<(u32, Vec<u8>)>, u32) {
        (self.string_data.clone(), self.next_data_offset)
    }

    /// Decoded view of the interned string table: each entry stored in
    /// `string_data` is `(data_offset, [u32 length prefix][utf8 bytes])`, while
    /// runtime string handles point at the utf8 bytes (offset + prefix size).
    pub(in crate::backend::direct_wasm) fn interned_string_texts(&self) -> Vec<(u32, String)> {
        self.string_data
            .iter()
            .filter_map(|(offset, data)| {
                let content = data.get(STRING_LENGTH_PREFIX_SIZE as usize..)?;
                let text = String::from_utf8(content.to_vec()).ok()?;
                Some((offset + STRING_LENGTH_PREFIX_SIZE, text))
            })
            .collect()
    }
}

impl CompilerState {
    pub(in crate::backend::direct_wasm) fn intern_string(&mut self, bytes: Vec<u8>) -> (u32, u32) {
        self.module_artifacts.intern_string(bytes)
    }

    pub(in crate::backend::direct_wasm) fn snapshot_module_data(
        &self,
    ) -> (Vec<(u32, Vec<u8>)>, u32) {
        self.module_artifacts.snapshot_data()
    }

    pub(in crate::backend::direct_wasm) fn reserve_zeroed_data(&mut self, len: u32) -> u32 {
        self.module_artifacts.reserve_zeroed_data(len)
    }
}
