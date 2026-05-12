use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn emit_runtime_descriptor_member_read(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> DirectResult<bool> {
        let Expression::Identifier(name) = object else {
            return Ok(false);
        };
        let resolved_name = self
            .resolve_current_local_binding(name)
            .map(|(resolved_name, _)| resolved_name)
            .unwrap_or_else(|| name.clone());
        let Some(descriptor) = self
            .state
            .speculation
            .static_semantics
            .objects
            .local_descriptor_bindings
            .get(&resolved_name)
        else {
            return Ok(false);
        };
        let Expression::String(property_name) = property else {
            return Ok(false);
        };

        let trace_descriptor_reads = std::env::var_os("AYY_TRACE_DESCRIPTOR_READS").is_some();
        if trace_descriptor_reads {
            eprintln!(
                "descriptor_read object={object:?} property={property:?} value={:?} configurable={} enumerable={} writable={:?} getter={:?} setter={:?}",
                descriptor.value,
                descriptor.configurable,
                descriptor.enumerable,
                descriptor.writable,
                descriptor.getter,
                descriptor.setter
            );
        }

        match property_name.as_str() {
            "value" => {
                if let Some(value) = descriptor.value.clone() {
                    if matches!(
                        &value,
                        Expression::Member {
                            object: value_object,
                            property: value_property,
                        } if value_object.as_ref() == object && value_property.as_ref() == property
                    ) {
                        if trace_descriptor_reads {
                            eprintln!(
                                "descriptor_read:self_reference object={object:?} property={property:?}"
                            );
                        }
                        return Ok(false);
                    }
                    self.emit_numeric_expression(&value)?;
                } else {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                }
                Ok(true)
            }
            "configurable" => {
                self.push_i32_const(if descriptor.configurable { 1 } else { 0 });
                Ok(true)
            }
            "enumerable" => {
                self.push_i32_const(if descriptor.enumerable { 1 } else { 0 });
                Ok(true)
            }
            "writable" => {
                if let Some(writable) = descriptor.writable {
                    self.push_i32_const(if writable { 1 } else { 0 });
                } else {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                }
                Ok(true)
            }
            "get" => {
                if let Some(getter) = descriptor.getter.clone() {
                    self.emit_numeric_expression(&getter)?;
                } else {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                }
                Ok(true)
            }
            "set" => {
                if let Some(setter) = descriptor.setter.clone() {
                    self.emit_numeric_expression(&setter)?;
                } else {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}
