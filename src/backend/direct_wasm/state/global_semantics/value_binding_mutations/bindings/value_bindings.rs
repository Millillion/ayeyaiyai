use super::super::super::super::super::*;
use crate::backend::direct_wasm::GlobalValueService;

impl GlobalValueService {
    fn remove_identifier_alias_binding(&mut self, alias_name: &str, value: &Expression) {
        let Expression::Identifier(source_name) = value else {
            return;
        };
        let Some(aliases) = self.identifier_alias_bindings.get_mut(source_name) else {
            return;
        };
        aliases.remove(alias_name);
        if aliases.is_empty() {
            self.identifier_alias_bindings.remove(source_name);
        }
    }

    fn add_identifier_alias_binding(&mut self, alias_name: &str, value: &Expression) {
        let Expression::Identifier(source_name) = value else {
            return;
        };
        if source_name == alias_name {
            return;
        }
        self.identifier_alias_bindings
            .entry(source_name.clone())
            .or_default()
            .insert(alias_name.to_string());
    }

    pub(in crate::backend::direct_wasm) fn clear_value_binding(&mut self, name: &str) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        if let Some(old_value) = self.value_bindings.remove(name) {
            self.remove_identifier_alias_binding(name, &old_value);
        }
    }

    pub(in crate::backend::direct_wasm) fn set_value_binding(
        &mut self,
        name: String,
        value: Expression,
    ) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        if let Some(old_value) = self.value_bindings.insert(name.clone(), value.clone()) {
            self.remove_identifier_alias_binding(&name, &old_value);
        }
        self.add_identifier_alias_binding(&name, &value);
    }

    pub(in crate::backend::direct_wasm) fn rebuild_identifier_alias_bindings(&mut self) {
        self.identifier_alias_bindings.clear();
        let alias_pairs = self
            .value_bindings
            .iter()
            .filter_map(|(alias_name, value)| {
                let Expression::Identifier(source_name) = value else {
                    return None;
                };
                (source_name != alias_name).then(|| (source_name.clone(), alias_name.clone()))
            })
            .collect::<Vec<_>>();
        for (source_name, alias_name) in alias_pairs {
            self.identifier_alias_bindings
                .entry(source_name)
                .or_default()
                .insert(alias_name);
        }
    }
}
