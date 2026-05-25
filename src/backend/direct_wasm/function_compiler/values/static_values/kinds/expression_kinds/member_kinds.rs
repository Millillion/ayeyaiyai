use super::*;

#[path = "member_kinds/builtin_members.rs"]
mod builtin_members;
#[path = "member_kinds/object_members.rs"]
mod object_members;
#[path = "member_kinds/special_members.rs"]
mod special_members;

impl<'a> FunctionCompiler<'a> {
    pub(super) fn infer_member_expression_kind(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<StaticValueKind> {
        if let Some(value) = self.resolve_static_member_getter_value_with_context(
            object,
            property,
            self.current_function_name(),
        ) {
            if self.expression_is_static_boxed_primitive_object(&value) {
                return Some(StaticValueKind::Object);
            }
            return self
                .infer_value_kind(&value)
                .or(Some(StaticValueKind::Unknown));
        }
        self.infer_special_member_kind(object, property)
            .or_else(|| self.infer_builtin_member_kind(object, property))
            .or_else(|| self.infer_object_member_kind(object, property))
            .or(Some(StaticValueKind::Unknown))
    }
}
