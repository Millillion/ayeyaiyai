use super::*;

#[path = "descriptor_lookup/arguments.rs"]
mod arguments;
#[path = "descriptor_lookup/call_lookup.rs"]
mod call_lookup;
#[path = "descriptor_lookup/function_properties.rs"]
mod function_properties;
#[path = "descriptor_lookup/identifier_lookup.rs"]
mod identifier_lookup;
#[path = "descriptor_lookup/object_properties.rs"]
mod object_properties;
#[path = "descriptor_lookup/state_lookup.rs"]
mod state_lookup;
#[path = "descriptor_lookup/top_level.rs"]
mod top_level;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn expression_reads_local_descriptor_binding_member(
        &self,
        expression: &Expression,
    ) -> bool {
        match expression {
            Expression::Member { object, property } => {
                if let (Expression::Identifier(name), Expression::String(property_name)) =
                    (object.as_ref(), property.as_ref())
                    && matches!(
                        property_name.as_str(),
                        "value" | "configurable" | "enumerable" | "writable" | "get" | "set"
                    )
                {
                    let resolved_name = self
                        .resolve_current_local_binding(name)
                        .map(|(resolved_name, _)| resolved_name)
                        .unwrap_or_else(|| name.clone());
                    if self
                        .state
                        .speculation
                        .static_semantics
                        .objects
                        .local_descriptor_bindings
                        .contains_key(&resolved_name)
                    {
                        return true;
                    }
                }

                self.expression_reads_local_descriptor_binding_member(object)
                    || self.expression_reads_local_descriptor_binding_member(property)
            }
            Expression::Unary { expression, .. }
            | Expression::Await(expression)
            | Expression::EnumerateKeys(expression)
            | Expression::GetIterator(expression)
            | Expression::IteratorClose(expression) => {
                self.expression_reads_local_descriptor_binding_member(expression)
            }
            Expression::Binary { left, right, .. } => {
                self.expression_reads_local_descriptor_binding_member(left)
                    || self.expression_reads_local_descriptor_binding_member(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.expression_reads_local_descriptor_binding_member(condition)
                    || self.expression_reads_local_descriptor_binding_member(then_expression)
                    || self.expression_reads_local_descriptor_binding_member(else_expression)
            }
            Expression::Sequence(expressions) => expressions.iter().any(|expression| {
                self.expression_reads_local_descriptor_binding_member(expression)
            }),
            Expression::Assign { value, .. } => {
                self.expression_reads_local_descriptor_binding_member(value)
            }
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.expression_reads_local_descriptor_binding_member(object)
                    || self.expression_reads_local_descriptor_binding_member(property)
                    || self.expression_reads_local_descriptor_binding_member(value)
            }
            Expression::AssignSuperMember { property, value } => {
                self.expression_reads_local_descriptor_binding_member(property)
                    || self.expression_reads_local_descriptor_binding_member(value)
            }
            Expression::Call { callee, arguments }
            | Expression::New { callee, arguments }
            | Expression::SuperCall { callee, arguments } => {
                self.expression_reads_local_descriptor_binding_member(callee)
                    || arguments.iter().any(|argument| match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.expression_reads_local_descriptor_binding_member(expression)
                        }
                    })
            }
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                    self.expression_reads_local_descriptor_binding_member(expression)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    self.expression_reads_local_descriptor_binding_member(key)
                        || self.expression_reads_local_descriptor_binding_member(value)
                }
                ObjectEntry::Getter { key, getter } => {
                    self.expression_reads_local_descriptor_binding_member(key)
                        || self.expression_reads_local_descriptor_binding_member(getter)
                }
                ObjectEntry::Setter { key, setter } => {
                    self.expression_reads_local_descriptor_binding_member(key)
                        || self.expression_reads_local_descriptor_binding_member(setter)
                }
                ObjectEntry::Spread(expression) => {
                    self.expression_reads_local_descriptor_binding_member(expression)
                }
            }),
            Expression::SuperMember { property } => {
                self.expression_reads_local_descriptor_binding_member(property)
            }
            Expression::Identifier(_)
            | Expression::Update { .. }
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::This
            | Expression::Sent
            | Expression::NewTarget => false,
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_descriptor_binding_from_expression(
        &self,
        expression: &Expression,
    ) -> Option<PropertyDescriptorBinding> {
        match expression {
            Expression::Identifier(name) => self.resolve_identifier_descriptor_binding(name),
            Expression::Call { callee, arguments } => {
                self.resolve_call_descriptor_binding(callee, arguments)
            }
            _ => None,
        }
    }
}
