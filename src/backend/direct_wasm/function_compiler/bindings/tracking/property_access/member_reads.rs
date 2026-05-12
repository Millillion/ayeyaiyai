use super::*;

mod getter_calls;
mod runtime_reads;
mod static_reads;

impl<'a> FunctionCompiler<'a> {
    pub(super) fn is_private_member_read_property(&self, property: &Expression) -> bool {
        matches!(
            self.resolve_property_key_expression(property)
                .unwrap_or_else(|| self.materialize_static_expression(property)),
            Expression::String(name) if name.starts_with("__ayy$private$")
        )
    }

    pub(in crate::backend::direct_wasm) fn emit_member_read_without_prelude(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> DirectResult<()> {
        let trace_member_reads = std::env::var_os("AYY_TRACE_MEMBER_READS").is_some();
        if trace_member_reads {
            eprintln!("member_read:start object={object:?} property={property:?}");
        }
        let static_array_property = if inline_summary_side_effect_free_expression(property)
            && !self.expression_depends_on_active_loop_assignment(property)
        {
            self.resolve_property_key_expression(property)
                .unwrap_or_else(|| self.materialize_static_expression(property))
        } else {
            property.clone()
        };

        if trace_member_reads {
            eprintln!(
                "member_read:static_property object={object:?} property={property:?} static={static_array_property:?}"
            );
        }
        let reads_descriptor_member =
            self.expression_reads_local_descriptor_binding_member(&Expression::Member {
                object: Box::new(object.clone()),
                property: Box::new(property.clone()),
            });
        if trace_member_reads {
            eprintln!(
                "member_read:descriptor_check object={object:?} property={property:?} reads={reads_descriptor_member}"
            );
        }
        let descriptor_read_emitted = reads_descriptor_member
            && self.emit_runtime_descriptor_member_read(object, property)?;
        if descriptor_read_emitted {
            if trace_member_reads {
                eprintln!("member_read:descriptor_hit object={object:?} property={property:?}");
            }
            if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                eprintln!(
                    "runtime_shadow_member_dispatch descriptor_early object={object:?} property={property:?}"
                );
            }
            return Ok(());
        }
        let skip_static_special_for_descriptor_member = reads_descriptor_member
            && self
                .resolve_iterator_step_binding_from_expression(object)
                .is_none();
        if trace_member_reads {
            eprintln!(
                "member_read:before_special object={object:?} property={property:?} skip={skip_static_special_for_descriptor_member}"
            );
        }
        if !skip_static_special_for_descriptor_member
            && self.emit_special_member_read_without_prelude(
                object,
                property,
                &static_array_property,
            )?
        {
            if trace_member_reads {
                eprintln!("member_read:special_hit object={object:?} property={property:?}");
            }
            if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                eprintln!(
                    "runtime_shadow_member_dispatch special object={object:?} property={property:?}"
                );
            }
            return Ok(());
        }
        if trace_member_reads {
            eprintln!("member_read:before_binding object={object:?} property={property:?}");
        }
        if self.emit_member_binding_read_without_prelude(object, property)? {
            if trace_member_reads {
                eprintln!("member_read:binding_hit object={object:?} property={property:?}");
            }
            if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                eprintln!(
                    "runtime_shadow_member_dispatch binding object={object:?} property={property:?}"
                );
            }
            return Ok(());
        }
        if trace_member_reads {
            eprintln!("member_read:before_runtime object={object:?} property={property:?}");
        }
        if self.emit_runtime_or_object_member_read_without_prelude(
            object,
            property,
            &static_array_property,
        )? {
            if trace_member_reads {
                eprintln!("member_read:runtime_hit object={object:?} property={property:?}");
            }
            if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                eprintln!(
                    "runtime_shadow_member_dispatch runtime object={object:?} property={property:?}"
                );
            }
            return Ok(());
        }
        if self.is_private_member_read_property(property) {
            return self.emit_named_error_throw("TypeError");
        }
        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        Ok(())
    }
}
