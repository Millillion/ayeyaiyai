use std::collections::BTreeSet;

use super::*;

fn push_wtf8_code_point(output: &mut String, code_point: u32) {
    if let Ok(unit) = u16::try_from(code_point)
        && let Some(sentinel) = js_surrogate_code_unit_to_sentinel(unit)
    {
        output.push(sentinel);
        return;
    }
    output.push(char::from_u32(code_point).unwrap_or(char::REPLACEMENT_CHARACTER));
}

fn lower_string_literal_value(string: &Str) -> String {
    let bytes = string.value.as_bytes();
    let mut output = String::new();
    let mut index = 0;
    while index < bytes.len() {
        let leading = bytes[index];
        if leading < 0x80 {
            output.push(char::from(leading));
            index += 1;
        } else if leading & 0xe0 == 0xc0 && index + 1 < bytes.len() {
            let code_point = (u32::from(leading & 0x1f) << 6) | u32::from(bytes[index + 1] & 0x3f);
            push_wtf8_code_point(&mut output, code_point);
            index += 2;
        } else if leading & 0xf0 == 0xe0 && index + 2 < bytes.len() {
            let code_point = (u32::from(leading & 0x0f) << 12)
                | (u32::from(bytes[index + 1] & 0x3f) << 6)
                | u32::from(bytes[index + 2] & 0x3f);
            push_wtf8_code_point(&mut output, code_point);
            index += 3;
        } else if leading & 0xf8 == 0xf0 && index + 3 < bytes.len() {
            let code_point = (u32::from(leading & 0x07) << 18)
                | (u32::from(bytes[index + 1] & 0x3f) << 12)
                | (u32::from(bytes[index + 2] & 0x3f) << 6)
                | u32::from(bytes[index + 3] & 0x3f);
            push_wtf8_code_point(&mut output, code_point);
            index += 4;
        } else {
            output.push(char::REPLACEMENT_CHARACTER);
            index += 1;
        }
    }
    output
}

fn ascii_hex_digit_value(byte: u8) -> Option<u32> {
    match byte {
        b'0'..=b'9' => Some(u32::from(byte - b'0')),
        b'a'..=b'f' => Some(u32::from(byte - b'a') + 10),
        b'A'..=b'F' => Some(u32::from(byte - b'A') + 10),
        _ => None,
    }
}

fn tagged_template_raw_has_invalid_escape(raw: &str) -> bool {
    let bytes = raw.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'\\' {
            index += 1;
            continue;
        }
        index += 1;
        if index >= bytes.len() {
            return false;
        }
        match bytes[index] {
            b'1'..=b'7' | b'8' | b'9' => return true,
            b'0' => {
                if bytes
                    .get(index + 1)
                    .is_some_and(|byte| byte.is_ascii_digit())
                {
                    return true;
                }
                index += 1;
            }
            b'x' => {
                if index + 2 >= bytes.len()
                    || ascii_hex_digit_value(bytes[index + 1]).is_none()
                    || ascii_hex_digit_value(bytes[index + 2]).is_none()
                {
                    return true;
                }
                index += 3;
            }
            b'u' => {
                if bytes.get(index + 1) == Some(&b'{') {
                    let mut cursor = index + 2;
                    let mut value = 0u32;
                    let mut digit_count = 0usize;
                    while cursor < bytes.len() && bytes[cursor] != b'}' {
                        let Some(digit) = ascii_hex_digit_value(bytes[cursor]) else {
                            return true;
                        };
                        value = value.saturating_mul(16).saturating_add(digit);
                        digit_count += 1;
                        cursor += 1;
                    }
                    if cursor >= bytes.len() || digit_count == 0 || value > 0x10ffff {
                        return true;
                    }
                    index = cursor + 1;
                } else {
                    if index + 4 >= bytes.len()
                        || bytes[index + 1..=index + 4]
                            .iter()
                            .any(|byte| ascii_hex_digit_value(*byte).is_none())
                    {
                        return true;
                    }
                    index += 5;
                }
            }
            b'\r' => {
                index += if bytes.get(index + 1) == Some(&b'\n') {
                    2
                } else {
                    1
                };
            }
            b'\n' => {
                index += 1;
            }
            _ => {
                index += 1;
            }
        }
    }
    false
}

impl Lowerer {
    pub(crate) fn lower_expression(&mut self, expression: &Expr) -> Result<Expression> {
        self.lower_expression_with_name_hint(expression, None)
    }

    pub(crate) fn lower_expression_with_name_hint(
        &mut self,
        expression: &Expr,
        name_hint: Option<&str>,
    ) -> Result<Expression> {
        if let Some(arguments) = console_log_arguments(expression) {
            return Ok(Expression::Call {
                callee: Box::new(Expression::Identifier("__ayyPrint".to_string())),
                arguments: arguments
                    .iter()
                    .map(|argument| {
                        let expression = self.lower_expression(&argument.expr)?;
                        Ok(if argument.spread.is_some() {
                            CallArgument::Spread(expression)
                        } else {
                            CallArgument::Expression(expression)
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
            });
        }

        match expression {
            Expr::Lit(Lit::Num(number)) => Ok(Expression::Number(number.value)),
            Expr::Lit(Lit::BigInt(bigint)) => Ok(Expression::BigInt(parse_bigint_literal(
                &bigint.value.to_string(),
            )?)),
            Expr::Lit(Lit::Str(string)) => {
                Ok(Expression::String(lower_string_literal_value(string)))
            }
            Expr::Lit(Lit::Bool(boolean)) => Ok(Expression::Bool(boolean.value)),
            Expr::Lit(Lit::Null(_)) => Ok(Expression::Null),
            Expr::MetaProp(meta_property) => match meta_property.kind {
                MetaPropKind::NewTarget => Ok(Expression::NewTarget),
                MetaPropKind::ImportMeta => Ok(Expression::Call {
                    callee: Box::new(Expression::Identifier("__ayyImportMeta".to_string())),
                    arguments: Vec::new(),
                }),
            },
            Expr::Lit(Lit::Regex(regex)) => Ok(Expression::Call {
                callee: Box::new(Expression::Identifier("RegExp".to_string())),
                arguments: vec![
                    CallArgument::Expression(Expression::String(regex.exp.to_string())),
                    CallArgument::Expression(Expression::String(regex.flags.to_string())),
                ],
            }),
            Expr::Tpl(template) => self.lower_template_expression(template),
            Expr::Array(array) => Ok(Expression::Array(
                array
                    .elems
                    .iter()
                    .map(|element| match element {
                        Some(element) => {
                            let expression = self.lower_expression(&element.expr)?;
                            Ok(if element.spread.is_some() {
                                ArrayElement::Spread(expression)
                            } else {
                                ArrayElement::Expression(expression)
                            })
                        }
                        None => Ok(ArrayElement::Expression(Expression::Undefined)),
                    })
                    .collect::<Result<Vec<_>>>()?,
            )),
            Expr::Object(object) => Ok(Expression::Object(
                object
                    .props
                    .iter()
                    .map(|property| self.lower_object_entry(property))
                    .collect::<Result<Vec<_>>>()?,
            )),
            Expr::Ident(identifier) => Ok(Expression::Identifier(
                self.resolve_binding_name(identifier.sym.as_ref()),
            )),
            Expr::PrivateName(private_name) => self.lower_private_name(private_name),
            Expr::This(_) => Ok(self.current_this_replacement().unwrap_or(Expression::This)),
            Expr::OptChain(optional_chain) => self.lower_optional_chain_expression(optional_chain),
            Expr::Member(member) => {
                let object = self.lower_expression(&member.obj)?;
                let property = self.lower_member_property(&member.prop)?;
                Ok(self.build_optional_member_expression(object, property, false))
            }
            Expr::SuperProp(super_property) => {
                let property = self.lower_super_property(super_property)?;
                if let Some(object) = self.current_super_member_replacement() {
                    Ok(Expression::Member {
                        object: Box::new(object),
                        property: Box::new(property),
                    })
                } else {
                    Ok(Expression::SuperMember {
                        property: Box::new(property),
                    })
                }
            }
            Expr::Paren(parenthesized) => {
                self.lower_expression_with_name_hint(&parenthesized.expr, name_hint)
            }
            Expr::Await(await_expression) => Ok(Expression::Await(Box::new(
                self.lower_expression_with_name_hint(&await_expression.arg, name_hint)?,
            ))),
            Expr::Unary(unary) => Ok(Expression::Unary {
                op: lower_unary_operator(unary.op)?,
                expression: Box::new(self.lower_expression(&unary.arg)?),
            }),
            Expr::Bin(binary) => Ok(Expression::Binary {
                op: lower_binary_operator(binary.op)?,
                left: Box::new(self.lower_expression(&binary.left)?),
                right: Box::new(self.lower_expression(&binary.right)?),
            }),
            Expr::Cond(conditional) => Ok(Expression::Conditional {
                condition: Box::new(self.lower_expression(&conditional.test)?),
                then_expression: Box::new(self.lower_expression(&conditional.cons)?),
                else_expression: Box::new(self.lower_expression(&conditional.alt)?),
            }),
            Expr::Seq(sequence) => Ok(Expression::Sequence(
                sequence
                    .exprs
                    .iter()
                    .map(|expression| self.lower_expression(expression))
                    .collect::<Result<Vec<_>>>()?,
            )),
            Expr::Assign(assignment) => {
                if assignment.op == AssignOp::Assign
                    && let swc_ecma_ast::AssignTarget::Pat(pattern) = &assignment.left
                {
                    let value = self.lower_expression(&assignment.right)?;
                    let pattern: Pat = pattern.clone().into();
                    return self.lower_assignment_pattern_expression(&pattern, value);
                }

                let target_name_hint = self.assignment_target_name_hint(&assignment.left);
                let target = self.lower_assignment_target(&assignment.left)?;
                let right = match target_name_hint.as_deref() {
                    Some(name_hint) => {
                        self.lower_expression_with_name_hint(&assignment.right, Some(name_hint))?
                    }
                    None => self.lower_expression(&assignment.right)?,
                };

                match assignment.op {
                    AssignOp::Assign => self.lower_assignment_expression(target, right),
                    AssignOp::AndAssign => self.lower_logical_assignment_expression(
                        target,
                        right,
                        LogicalAssignmentKind::And,
                    ),
                    AssignOp::OrAssign => self.lower_logical_assignment_expression(
                        target,
                        right,
                        LogicalAssignmentKind::Or,
                    ),
                    AssignOp::NullishAssign => self.lower_logical_assignment_expression(
                        target,
                        right,
                        LogicalAssignmentKind::Nullish,
                    ),
                    operator => {
                        let binary_operator = lower_binary_operator(
                            operator
                                .to_update()
                                .context("unsupported assignment operator")?,
                        )?;
                        let value = match &target {
                            AssignmentTarget::Identifier(name) => Expression::Binary {
                                op: binary_operator,
                                left: Box::new(Expression::Identifier(name.clone())),
                                right: Box::new(right),
                            },
                            AssignmentTarget::Member { object, property } => Expression::Binary {
                                op: binary_operator,
                                left: Box::new(Expression::Member {
                                    object: Box::new(object.clone()),
                                    property: Box::new(property.clone()),
                                }),
                                right: Box::new(right),
                            },
                            AssignmentTarget::SuperMember { property } => Expression::Binary {
                                op: binary_operator,
                                left: Box::new(Expression::SuperMember {
                                    property: Box::new(property.clone()),
                                }),
                                right: Box::new(right),
                            },
                        };

                        self.lower_assignment_expression(target, value)
                    }
                }
            }
            Expr::Call(call) => match &call.callee {
                Callee::Expr(callee) => {
                    if let Some(expression) = self.lower_static_direct_eval_expression(call)? {
                        return Ok(expression);
                    }

                    let callee = self.lower_expression(callee)?;
                    let arguments = call
                        .args
                        .iter()
                        .map(|argument| {
                            let expression = self.lower_expression(&argument.expr)?;
                            Ok(if argument.spread.is_some() {
                                CallArgument::Spread(expression)
                            } else {
                                CallArgument::Expression(expression)
                            })
                        })
                        .collect::<Result<Vec<_>>>()?;
                    Ok(self.build_optional_call_expression(callee, arguments, false))
                }
                Callee::Super(_) => {
                    let super_name = self
                        .constructor_super_stack
                        .last()
                        .and_then(|name| name.clone())
                        .context("`super()` is only supported in derived constructors")?;
                    Ok(Expression::SuperCall {
                        callee: Box::new(Expression::Identifier(super_name)),
                        arguments: call
                            .args
                            .iter()
                            .map(|argument| {
                                let expression = self.lower_expression(&argument.expr)?;
                                Ok(if argument.spread.is_some() {
                                    CallArgument::Spread(expression)
                                } else {
                                    CallArgument::Expression(expression)
                                })
                            })
                            .collect::<Result<Vec<_>>>()?,
                    })
                }
                Callee::Import(_) => self.lower_dynamic_import_expression(call),
            },
            Expr::TaggedTpl(tagged_template) => {
                let template_site_id = self.next_template_object_id;
                self.next_template_object_id += 1;
                let template_site_key = format!("template-site:{template_site_id}");
                let cooked = Expression::Array(
                    tagged_template
                        .tpl
                        .quasis
                        .iter()
                        .map(|quasi| {
                            let raw = quasi.raw.to_string();
                            let value = if tagged_template_raw_has_invalid_escape(&raw) {
                                Expression::Undefined
                            } else {
                                quasi
                                    .cooked
                                    .as_ref()
                                    .and_then(|value| value.as_str())
                                    .map(|value| Expression::String(value.to_string()))
                                    .unwrap_or(Expression::Undefined)
                            };
                            Ok(ArrayElement::Expression(value))
                        })
                        .collect::<Result<Vec<_>>>()?,
                );
                let raw = Expression::Array(
                    tagged_template
                        .tpl
                        .quasis
                        .iter()
                        .map(|quasi| {
                            Ok(ArrayElement::Expression(Expression::String(
                                template_raw_text(&quasi.raw.to_string()),
                            )))
                        })
                        .collect::<Result<Vec<_>>>()?,
                );
                Ok(Expression::Call {
                    callee: Box::new(self.lower_expression(&tagged_template.tag)?),
                    arguments: std::iter::once(Ok(CallArgument::Expression(Expression::Call {
                        callee: Box::new(Expression::Identifier("__ayyTemplateObject".to_string())),
                        arguments: vec![
                            CallArgument::Expression(Expression::String(template_site_key)),
                            CallArgument::Expression(cooked),
                            CallArgument::Expression(raw),
                        ],
                    })))
                    .chain(tagged_template.tpl.exprs.iter().map(|expression| {
                        self.lower_expression(expression)
                            .map(CallArgument::Expression)
                    }))
                    .collect::<Result<Vec<_>>>()?,
                })
            }
            Expr::New(new_expression) => Ok(Expression::New {
                callee: Box::new(self.lower_expression(&new_expression.callee)?),
                arguments: new_expression
                    .args
                    .as_deref()
                    .unwrap_or(&[])
                    .iter()
                    .map(|argument| {
                        let expression = self.lower_expression(&argument.expr)?;
                        Ok(if argument.spread.is_some() {
                            CallArgument::Spread(expression)
                        } else {
                            CallArgument::Expression(expression)
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
            }),
            Expr::Fn(function_expression) => {
                self.lower_function_expression(function_expression, name_hint)
            }
            Expr::Class(class_expression) => {
                self.lower_class_expression(class_expression, name_hint)
            }
            Expr::Arrow(arrow_expression) => {
                self.lower_arrow_expression(arrow_expression, name_hint)
            }
            Expr::Update(update) => {
                if let Expr::Ident(identifier) = &*update.arg {
                    let name = self.resolve_binding_name(identifier.sym.as_ref());
                    return Ok(Expression::Update {
                        name,
                        op: lower_update_operator(update.op),
                        prefix: update.prefix,
                    });
                }

                let op = lower_update_operator(update.op);
                let target = self.lower_update_assignment_target(&update.arg)?;
                if update.prefix {
                    self.lower_prefix_update_assignment_expression(target, op)
                } else {
                    self.lower_postfix_update_assignment_expression(target, op)
                }
            }
            _ => bail!("unsupported expression: {expression:?}"),
        }
    }

    pub(super) fn lower_update_assignment_target(
        &mut self,
        target: &Expr,
    ) -> Result<AssignmentTarget> {
        match target {
            Expr::Ident(identifier) => Ok(AssignmentTarget::Identifier(
                self.resolve_binding_name(identifier.sym.as_ref()),
            )),
            Expr::Member(member) => Ok(AssignmentTarget::Member {
                object: self.lower_expression(&member.obj)?,
                property: self.lower_member_property(&member.prop)?,
            }),
            Expr::SuperProp(super_property) => Ok(AssignmentTarget::SuperMember {
                property: self.lower_super_property(super_property)?,
            }),
            Expr::Paren(parenthesized) => self.lower_update_assignment_target(&parenthesized.expr),
            _ => bail!("unsupported update target"),
        }
    }

    pub(super) fn update_assignment_value(target: &AssignmentTarget, op: UpdateOp) -> Expression {
        Self::updated_numeric_value(target.as_expression(), op)
    }

    fn updated_numeric_value(value: Expression, op: UpdateOp) -> Expression {
        Expression::Binary {
            op: match op {
                UpdateOp::Increment => BinaryOp::Add,
                UpdateOp::Decrement => BinaryOp::Subtract,
            },
            left: Box::new(Expression::Unary {
                op: UnaryOp::Plus,
                expression: Box::new(value),
            }),
            right: Box::new(Expression::Number(1.0)),
        }
    }

    fn postfix_updated_value(previous_name: &str, op: UpdateOp) -> Expression {
        Expression::Binary {
            op: match op {
                UpdateOp::Increment => BinaryOp::Add,
                UpdateOp::Decrement => BinaryOp::Subtract,
            },
            left: Box::new(Expression::Identifier(previous_name.to_string())),
            right: Box::new(Expression::Number(1.0)),
        }
    }

    fn postfix_previous_numeric_value(value: Expression) -> Expression {
        Expression::Unary {
            op: UnaryOp::Plus,
            expression: Box::new(value),
        }
    }

    pub(super) fn lower_prefix_update_assignment_expression(
        &mut self,
        target: AssignmentTarget,
        op: UpdateOp,
    ) -> Result<Expression> {
        match target {
            AssignmentTarget::Identifier(name) => Ok(Expression::Update {
                name,
                op,
                prefix: true,
            }),
            AssignmentTarget::Member { object, property } => {
                let object_name = self.fresh_temporary_name("target_object");
                let property_name = self.fresh_temporary_name("target_property");
                let cached_target = AssignmentTarget::Member {
                    object: Expression::Identifier(object_name.clone()),
                    property: Expression::Identifier(property_name.clone()),
                };
                let value = Self::update_assignment_value(&cached_target, op);
                let assignment = cached_target.into_expression(value);
                Ok(Expression::Sequence(vec![
                    Expression::Assign {
                        name: object_name,
                        value: Box::new(object),
                    },
                    Expression::Assign {
                        name: property_name,
                        value: Box::new(property),
                    },
                    assignment,
                ]))
            }
            AssignmentTarget::SuperMember { property } => {
                let target = AssignmentTarget::SuperMember { property };
                let value = Self::update_assignment_value(&target, op);
                Ok(target.into_expression(value))
            }
        }
    }

    pub(super) fn lower_postfix_update_assignment_expression(
        &mut self,
        target: AssignmentTarget,
        op: UpdateOp,
    ) -> Result<Expression> {
        let previous_name = self.fresh_temporary_name("postfix_previous");

        match target {
            AssignmentTarget::Identifier(name) => Ok(Expression::Update {
                name,
                op,
                prefix: false,
            }),
            AssignmentTarget::Member { object, property } => {
                let object_name = self.fresh_temporary_name("target_object");
                let property_name = self.fresh_temporary_name("target_property");
                let cached_target = AssignmentTarget::Member {
                    object: Expression::Identifier(object_name.clone()),
                    property: Expression::Identifier(property_name.clone()),
                };
                let current = cached_target.as_expression();
                let assignment =
                    cached_target.into_expression(Self::postfix_updated_value(&previous_name, op));
                Ok(Expression::Sequence(vec![
                    Expression::Assign {
                        name: object_name,
                        value: Box::new(object),
                    },
                    Expression::Assign {
                        name: property_name,
                        value: Box::new(property),
                    },
                    Expression::Assign {
                        name: previous_name.clone(),
                        value: Box::new(Self::postfix_previous_numeric_value(current)),
                    },
                    assignment,
                    Expression::Identifier(previous_name),
                ]))
            }
            AssignmentTarget::SuperMember { property } => {
                let current = Expression::SuperMember {
                    property: Box::new(property.clone()),
                };
                let assignment = Expression::AssignSuperMember {
                    property: Box::new(property),
                    value: Box::new(Self::postfix_updated_value(&previous_name, op)),
                };
                Ok(Expression::Sequence(vec![
                    Expression::Assign {
                        name: previous_name.clone(),
                        value: Box::new(Self::postfix_previous_numeric_value(current)),
                    },
                    assignment,
                    Expression::Identifier(previous_name),
                ]))
            }
        }
    }

    pub(super) fn lower_assignment_expression(
        &mut self,
        target: AssignmentTarget,
        value: Expression,
    ) -> Result<Expression> {
        Ok(target.into_expression(value))
    }

    pub(super) fn lower_logical_assignment_expression(
        &mut self,
        target: AssignmentTarget,
        right: Expression,
        kind: LogicalAssignmentKind,
    ) -> Result<Expression> {
        if let AssignmentTarget::Member { object, property } = target {
            let object_name = self.fresh_temporary_name("target_object");
            let property_name = self.fresh_temporary_name("target_property");
            let cached_target = AssignmentTarget::Member {
                object: Expression::Identifier(object_name.clone()),
                property: Expression::Identifier(property_name.clone()),
            };
            let current = cached_target.as_expression();
            let assignment = self.lower_assignment_expression(cached_target, right)?;
            let expression =
                Self::logical_assignment_expression_from_parts(current, assignment, kind);
            return Ok(Expression::Sequence(vec![
                Expression::Assign {
                    name: object_name,
                    value: Box::new(object),
                },
                Expression::Assign {
                    name: property_name,
                    value: Box::new(property),
                },
                expression,
            ]));
        }

        let current = target.as_expression();
        let assignment = self.lower_assignment_expression(target, right)?;
        Ok(Self::logical_assignment_expression_from_parts(
            current, assignment, kind,
        ))
    }

    fn logical_assignment_expression_from_parts(
        current: Expression,
        assignment: Expression,
        kind: LogicalAssignmentKind,
    ) -> Expression {
        match kind {
            LogicalAssignmentKind::And => Expression::Binary {
                op: BinaryOp::LogicalAnd,
                left: Box::new(current),
                right: Box::new(assignment),
            },
            LogicalAssignmentKind::Or => Expression::Binary {
                op: BinaryOp::LogicalOr,
                left: Box::new(current),
                right: Box::new(assignment),
            },
            LogicalAssignmentKind::Nullish => {
                let not_undefined = Expression::Binary {
                    op: BinaryOp::NotEqual,
                    left: Box::new(current.clone()),
                    right: Box::new(Expression::Undefined),
                };
                let not_null = Expression::Binary {
                    op: BinaryOp::NotEqual,
                    left: Box::new(current.clone()),
                    right: Box::new(Expression::Null),
                };

                Expression::Conditional {
                    condition: Box::new(Expression::Binary {
                        op: BinaryOp::LogicalAnd,
                        left: Box::new(not_undefined),
                        right: Box::new(not_null),
                    }),
                    then_expression: Box::new(current),
                    else_expression: Box::new(assignment),
                }
            }
        }
    }

    pub(crate) fn lower_object_entry(&mut self, property: &PropOrSpread) -> Result<ObjectEntry> {
        match property {
            PropOrSpread::Spread(spread) => {
                Ok(ObjectEntry::Spread(self.lower_expression(&spread.expr)?))
            }
            PropOrSpread::Prop(property) => match &**property {
                Prop::Shorthand(identifier) => {
                    let key = Expression::String(identifier.sym.to_string());
                    Ok(ObjectEntry::Data {
                        key: if identifier.sym == *"__proto__" {
                            Expression::Sequence(vec![key])
                        } else {
                            key
                        },
                        value: Expression::Identifier(identifier.sym.to_string()),
                    })
                }
                Prop::Method(method) => {
                    let key = self.lower_prop_name(&method.key)?;
                    self.lower_object_method_entry_with_key(method, key)
                }
                Prop::Getter(getter) => {
                    let key = self.lower_prop_name(&getter.key)?;
                    self.lower_object_getter_entry_with_key(getter, key)
                }
                Prop::Setter(setter) => {
                    let key = self.lower_prop_name(&setter.key)?;
                    self.lower_object_setter_entry_with_key(setter, key)
                }
                Prop::KeyValue(property) => {
                    let name_hint = self.object_prop_name_hint(&property.key);
                    Ok(ObjectEntry::Data {
                        key: self.lower_prop_name(&property.key)?,
                        value: self.lower_expression_with_name_hint(
                            &property.value,
                            name_hint.as_deref(),
                        )?,
                    })
                }
                _ => {
                    bail!(
                        "only shorthand, key/value, method, getter, and setter object properties are supported"
                    )
                }
            },
        }
    }

    pub(super) fn object_prop_name_hint(&self, name: &PropName) -> Option<String> {
        match name {
            PropName::Ident(identifier) => Some(identifier.sym.to_string()),
            PropName::Str(string) => Some(string.value.to_string_lossy().into_owned()),
            PropName::Num(number) => Some(number.value.to_string()),
            PropName::BigInt(bigint) => Some(bigint.value.to_string()),
            PropName::Computed(computed) => match computed.expr.as_ref() {
                Expr::Lit(Lit::Str(string)) => Some(string.value.to_string_lossy().into_owned()),
                Expr::Lit(Lit::Num(number)) => Some(number.value.to_string()),
                Expr::Lit(Lit::BigInt(bigint)) => Some(bigint.value.to_string()),
                Expr::Lit(Lit::Bool(boolean)) => Some(boolean.value.to_string()),
                Expr::Lit(Lit::Null(_)) => Some("null".to_string()),
                _ => None,
            },
        }
    }

    pub(crate) fn lower_object_method_entry_with_key(
        &mut self,
        method: &swc_ecma_ast::MethodProp,
        key: Expression,
    ) -> Result<ObjectEntry> {
        self.next_function_expression_id += 1;
        let generated_name = format!("__ayy_method_{}", self.next_function_expression_id);
        let (params, body, captured_private_brand_bindings) =
            self.lower_function_parts(&method.function, &[])?;

        self.functions.push(FunctionDeclaration {
            name: generated_name.clone(),
            top_level_binding: None,
            params,
            body,
            register_global: false,
            kind: lower_function_kind(method.function.is_generator, method.function.is_async),
            self_binding: None,
            mapped_arguments: self.function_has_mapped_arguments(&method.function),
            strict: self.function_strict_mode(&method.function),
            lexical_this: false,
            constructible: false,
            derived_constructor: false,
            direct_eval_in_class_field_initializer: self.class_field_initializer_depth > 0,
            length: expected_argument_count(
                method
                    .function
                    .params
                    .iter()
                    .map(|parameter| &parameter.pat),
            ),
            synthetic_capture_bindings: captured_private_brand_bindings
                .into_iter()
                .collect::<Vec<_>>(),
            immutable_class_bindings: self.current_immutable_class_bindings(),
            private_brand_binding: None,
        });

        Ok(ObjectEntry::Data {
            key,
            value: Expression::Identifier(generated_name),
        })
    }

    pub(crate) fn lower_object_getter_entry_with_key(
        &mut self,
        getter: &swc_ecma_ast::GetterProp,
        key: Expression,
    ) -> Result<ObjectEntry> {
        self.next_function_expression_id += 1;
        let generated_name = format!("__ayy_getter_{}", self.next_function_expression_id);
        let body = getter.body.as_ref().context("getters must have a body")?;
        let strict_mode =
            self.current_strict_mode() || script_has_use_strict_directive(&body.stmts);
        self.strict_modes.push(strict_mode);
        self.pending_private_brand_captures.push(BTreeSet::new());
        let lowered_body = self.with_this_replacement(None, |lowerer| {
            lowerer.with_super_member_replacement(None, |lowerer| {
                lowerer.lower_statements(&body.stmts, true, false)
            })
        });
        let captured_private_brand_bindings = self
            .pending_private_brand_captures
            .pop()
            .expect("getter private brand capture collector should exist");
        self.strict_modes.pop();
        let lowered_body = lowered_body?;

        self.functions.push(FunctionDeclaration {
            name: generated_name.clone(),
            top_level_binding: None,
            params: Vec::new(),
            body: lowered_body,
            register_global: false,
            kind: FunctionKind::Ordinary,
            self_binding: None,
            mapped_arguments: false,
            strict: strict_mode,
            lexical_this: false,
            constructible: false,
            derived_constructor: false,
            direct_eval_in_class_field_initializer: self.class_field_initializer_depth > 0,
            length: 0,
            synthetic_capture_bindings: captured_private_brand_bindings
                .into_iter()
                .collect::<Vec<_>>(),
            immutable_class_bindings: self.current_immutable_class_bindings(),
            private_brand_binding: None,
        });

        Ok(ObjectEntry::Getter {
            key,
            getter: Expression::Identifier(generated_name),
        })
    }

    pub(crate) fn lower_object_setter_entry_with_key(
        &mut self,
        setter: &swc_ecma_ast::SetterProp,
        key: Expression,
    ) -> Result<ObjectEntry> {
        self.next_function_expression_id += 1;
        let generated_name = format!("__ayy_setter_{}", self.next_function_expression_id);
        let body = setter.body.as_ref().context("setters must have a body")?;
        let strict_mode =
            self.current_strict_mode() || script_has_use_strict_directive(&body.stmts);
        self.strict_modes.push(strict_mode);
        self.pending_private_brand_captures.push(BTreeSet::new());
        let lowered = self.with_this_replacement(None, |lowerer| {
            lowerer.with_super_member_replacement(None, |lowerer| {
                let (params, mut param_setup) = lower_parameter(lowerer, &setter.param)?;
                let mut lowered_body = lowerer.lower_statements(&body.stmts, true, false)?;
                lowered_body.splice(0..0, param_setup.drain(..));
                Ok((params, lowered_body))
            })
        });
        let captured_private_brand_bindings = self
            .pending_private_brand_captures
            .pop()
            .expect("setter private brand capture collector should exist");
        self.strict_modes.pop();
        let (params, lowered_body) = lowered?;

        self.functions.push(FunctionDeclaration {
            name: generated_name.clone(),
            top_level_binding: None,
            params: vec![params],
            body: lowered_body,
            register_global: false,
            kind: FunctionKind::Ordinary,
            self_binding: None,
            mapped_arguments: false,
            strict: strict_mode,
            lexical_this: false,
            constructible: false,
            derived_constructor: false,
            direct_eval_in_class_field_initializer: self.class_field_initializer_depth > 0,
            length: expected_argument_count(std::iter::once(setter.param.as_ref())),
            synthetic_capture_bindings: captured_private_brand_bindings
                .into_iter()
                .collect::<Vec<_>>(),
            immutable_class_bindings: self.current_immutable_class_bindings(),
            private_brand_binding: None,
        });

        Ok(ObjectEntry::Setter {
            key,
            setter: Expression::Identifier(generated_name),
        })
    }

    pub(crate) fn lower_template_expression(
        &mut self,
        template: &swc_ecma_ast::Tpl,
    ) -> Result<Expression> {
        let expressions = template
            .exprs
            .iter()
            .map(|expression| self.lower_expression(expression))
            .collect::<Result<Vec<_>>>()?;
        self.build_template_expression(template, &expressions)
    }

    pub(crate) fn lower_template_expression_with_substitution(
        &mut self,
        template: &swc_ecma_ast::Tpl,
        index: usize,
        substitution: Expression,
    ) -> Result<Expression> {
        let mut expressions = Vec::with_capacity(template.exprs.len());
        for (expression_index, expression) in template.exprs.iter().enumerate() {
            if expression_index == index {
                expressions.push(substitution.clone());
            } else {
                expressions.push(self.lower_expression(expression)?);
            }
        }
        self.build_template_expression(template, &expressions)
    }

    pub(crate) fn build_template_expression(
        &mut self,
        template: &swc_ecma_ast::Tpl,
        expressions: &[Expression],
    ) -> Result<Expression> {
        for quasi in &template.quasis {
            let raw = quasi.raw.to_string();
            if quasi.cooked.is_none() || tagged_template_raw_has_invalid_escape(&raw) {
                bail!("invalid escape sequence in untagged template literal");
            }
        }

        let mut parts = Vec::new();
        for (index, quasi) in template.quasis.iter().enumerate() {
            parts.push(Expression::String(template_quasi_text(quasi)?));
            if let Some(expression) = expressions.get(index) {
                parts.push(expression.clone());
            }
        }

        let mut expression = parts
            .into_iter()
            .reduce(|left, right| Expression::Binary {
                op: BinaryOp::Add,
                left: Box::new(left),
                right: Box::new(right),
            })
            .unwrap_or(Expression::String(String::new()));
        if !matches!(expression, Expression::String(_)) {
            expression = Expression::Binary {
                op: BinaryOp::Add,
                left: Box::new(Expression::String(String::new())),
                right: Box::new(expression),
            };
        }
        Ok(expression)
    }

    pub(crate) fn lower_prop_name(&mut self, name: &PropName) -> Result<Expression> {
        Ok(match name {
            PropName::Ident(identifier) => Expression::String(identifier.sym.to_string()),
            PropName::Str(string) => {
                Expression::String(string.value.to_string_lossy().into_owned())
            }
            PropName::Num(number) => Expression::Number(number.value),
            PropName::BigInt(bigint) => Expression::String(bigint.value.to_string()),
            PropName::Computed(computed) => {
                Expression::Sequence(vec![self.lower_expression(&computed.expr)?])
            }
        })
    }

    fn lower_optional_chain_expression(
        &mut self,
        optional_chain: &swc_ecma_ast::OptChainExpr,
    ) -> Result<Expression> {
        match optional_chain.base.as_ref() {
            swc_ecma_ast::OptChainBase::Member(member) => {
                let object = self.lower_expression(&member.obj)?;
                let property = self.lower_member_property(&member.prop)?;
                Ok(
                    self.build_optional_member_expression(
                        object,
                        property,
                        optional_chain.optional,
                    ),
                )
            }
            swc_ecma_ast::OptChainBase::Call(call) => {
                let callee = self.lower_expression(&call.callee)?;
                let arguments = call
                    .args
                    .iter()
                    .map(|argument| {
                        let expression = self.lower_expression(&argument.expr)?;
                        Ok(if argument.spread.is_some() {
                            CallArgument::Spread(expression)
                        } else {
                            CallArgument::Expression(expression)
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                Ok(self.build_optional_call_expression(callee, arguments, optional_chain.optional))
            }
        }
    }

    fn build_optional_chain_nullish_check(&self, expression: &Expression) -> Expression {
        Expression::Binary {
            op: BinaryOp::LogicalOr,
            left: Box::new(Expression::Binary {
                op: BinaryOp::Equal,
                left: Box::new(expression.clone()),
                right: Box::new(Expression::Null),
            }),
            right: Box::new(Expression::Binary {
                op: BinaryOp::Equal,
                left: Box::new(expression.clone()),
                right: Box::new(Expression::Undefined),
            }),
        }
    }

    fn try_build_optional_chain_continuation<F>(
        expression: &Expression,
        continuation: F,
    ) -> Option<Expression>
    where
        F: FnOnce(Expression) -> Expression,
    {
        if let Expression::Sequence(expressions) = expression
            && let [
                Expression::Assign { name, value },
                Expression::Conditional {
                    condition,
                    then_expression,
                    else_expression,
                    ..
                },
            ] = expressions.as_slice()
            && name.starts_with("__ayy_optional_base_")
            && matches!(then_expression.as_ref(), Expression::Undefined)
        {
            return Some(Expression::Sequence(vec![
                Expression::Assign {
                    name: name.clone(),
                    value: value.clone(),
                },
                Expression::Conditional {
                    condition: condition.clone(),
                    then_expression: then_expression.clone(),
                    else_expression: Box::new(continuation(else_expression.as_ref().clone())),
                },
            ]));
        }

        if let Expression::Conditional {
            condition,
            then_expression,
            else_expression,
        } = expression
            && matches!(then_expression.as_ref(), Expression::Undefined)
        {
            return Some(Expression::Conditional {
                condition: condition.clone(),
                then_expression: then_expression.clone(),
                else_expression: Box::new(continuation(else_expression.as_ref().clone())),
            });
        }

        None
    }

    fn build_optional_member_expression(
        &mut self,
        object: Expression,
        property: Expression,
        optional: bool,
    ) -> Expression {
        if let Some(expression) = Self::try_build_optional_chain_continuation(&object, |object| {
            self.build_optional_member_expression(object, property.clone(), optional)
        }) {
            return expression;
        }

        if optional {
            let object_name = self.fresh_temporary_name("optional_base");
            let object_reference = Expression::Identifier(object_name.clone());
            let member = Expression::Member {
                object: Box::new(object_reference.clone()),
                property: Box::new(property),
            };
            Expression::Sequence(vec![
                Expression::Assign {
                    name: object_name,
                    value: Box::new(object),
                },
                Expression::Conditional {
                    condition: Box::new(self.build_optional_chain_nullish_check(&object_reference)),
                    then_expression: Box::new(Expression::Undefined),
                    else_expression: Box::new(member),
                },
            ])
        } else {
            let member = Expression::Member {
                object: Box::new(object),
                property: Box::new(property),
            };
            member
        }
    }

    fn build_optional_call_expression(
        &self,
        callee: Expression,
        arguments: Vec<CallArgument>,
        optional: bool,
    ) -> Expression {
        if let Some(expression) = Self::try_build_optional_chain_continuation(&callee, |callee| {
            self.build_optional_call_expression(callee, arguments.clone(), optional)
        }) {
            return expression;
        }

        let call_callee =
            if optional && matches!(callee, Expression::Identifier(ref name) if name == "eval") {
                Expression::Sequence(vec![callee.clone()])
            } else {
                callee.clone()
            };
        let call = Expression::Call {
            callee: Box::new(call_callee),
            arguments,
        };
        if optional {
            Expression::Conditional {
                condition: Box::new(self.build_optional_chain_nullish_check(&callee)),
                then_expression: Box::new(Expression::Undefined),
                else_expression: Box::new(call),
            }
        } else {
            call
        }
    }

    pub(crate) fn lower_member_property(&mut self, property: &MemberProp) -> Result<Expression> {
        Ok(match property {
            MemberProp::Ident(identifier) => Expression::String(identifier.sym.to_string()),
            MemberProp::Computed(computed) => {
                let expression = self.lower_expression(&computed.expr)?;
                Self::member_property_value_expression(expression)
            }
            MemberProp::PrivateName(private_name) => self.lower_private_name(private_name)?,
        })
    }

    pub(crate) fn lower_super_property(&mut self, property: &SuperPropExpr) -> Result<Expression> {
        Ok(match &property.prop {
            SuperProp::Ident(identifier) => Expression::String(identifier.sym.to_string()),
            SuperProp::Computed(computed) => {
                let expression = self.lower_expression(&computed.expr)?;
                Self::member_property_value_expression(expression)
            }
        })
    }

    fn member_property_value_expression(expression: Expression) -> Expression {
        expression
    }

    fn lower_static_direct_eval_expression(
        &mut self,
        call: &swc_ecma_ast::CallExpr,
    ) -> Result<Option<Expression>> {
        if call.args.len() != 1 || call.args[0].spread.is_some() {
            return Ok(None);
        }
        let Callee::Expr(callee) = &call.callee else {
            return Ok(None);
        };
        let Expr::Ident(identifier) = &**callee else {
            return Ok(None);
        };
        if identifier.sym.as_ref() != "eval" {
            return Ok(None);
        }
        if self
            .binding_scopes
            .iter()
            .any(|scope| scope.names.iter().any(|name| name == "eval"))
        {
            return Ok(None);
        }
        let Expr::Lit(Lit::Str(source)) = &*call.args[0].expr else {
            return Ok(None);
        };
        let source_text = source.value.to_string_lossy();
        if source_text.contains('`') {
            return Ok(None);
        }
        let Ok(SwcProgram::Script(script)) = parse_script_program_source(&source_text) else {
            return Ok(None);
        };
        let Some(expression) = single_static_eval_expression_statement(&script.body) else {
            return Ok(None);
        };

        self.lower_expression(expression).map(Some)
    }

    pub(crate) fn try_lower_top_level_this_member_update(
        &mut self,
        expression: &Expr,
    ) -> Result<Option<String>> {
        if self.module_mode || self.strict_modes.len() != 1 {
            return Ok(None);
        }

        let Expr::Member(member) = expression else {
            return Ok(None);
        };
        if !matches!(member.obj.as_ref(), Expr::This(_)) {
            return Ok(None);
        }

        let Some(name) = static_member_property_name(&member.prop) else {
            return Ok(None);
        };
        Ok(Some(self.resolve_binding_name(&name)))
    }
}

fn single_static_eval_expression_statement(statements: &[Stmt]) -> Option<&Expr> {
    let mut completion_expression = None;
    for statement in statements {
        match statement {
            Stmt::Empty(_) => {}
            Stmt::Expr(expression) if completion_expression.is_none() => {
                completion_expression = Some(expression.expr.as_ref());
            }
            _ => return None,
        }
    }

    completion_expression
}
