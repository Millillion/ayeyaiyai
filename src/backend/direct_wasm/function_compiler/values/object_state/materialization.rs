use super::*;

thread_local! {
    static ACTIVE_MATERIALIZATION_SHAPES: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

const MATERIALIZATION_SHAPE_DEPTH_LIMIT: usize = 16;
const MATERIALIZATION_SHAPE_NODE_LIMIT: usize = 256;
const MATERIALIZATION_SHAPE_TEXT_LIMIT: usize = 48;

struct StructuralMaterializationGuard {
    key: String,
}

impl Drop for StructuralMaterializationGuard {
    fn drop(&mut self) {
        ACTIVE_MATERIALIZATION_SHAPES.with(|active| {
            active.borrow_mut().remove(&self.key);
        });
    }
}

#[path = "materialization/compound.rs"]
mod compound;
#[path = "materialization/identifiers.rs"]
mod identifiers;
#[path = "materialization/members.rs"]
mod members;

fn push_limited_shape_text(output: &mut String, text: &str) {
    let mut truncated = false;
    for (index, character) in text.chars().enumerate() {
        if index >= MATERIALIZATION_SHAPE_TEXT_LIMIT {
            truncated = true;
            break;
        }
        output.push(character);
    }
    if truncated {
        output.push('~');
    }
}

fn push_materialization_shape_key(
    expression: &Expression,
    output: &mut String,
    depth: usize,
    budget: &mut usize,
) {
    if *budget == 0 {
        output.push_str("...");
        return;
    }
    *budget -= 1;

    if depth == 0 {
        output.push('#');
        return;
    }

    match expression {
        Expression::Number(value) => {
            output.push_str("num:");
            output.push_str(&value.to_bits().to_string());
        }
        Expression::BigInt(text) => {
            output.push_str("big:");
            push_limited_shape_text(output, text);
        }
        Expression::String(text) => {
            output.push_str("str:");
            push_limited_shape_text(output, text);
        }
        Expression::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
        Expression::Null => output.push_str("null"),
        Expression::Undefined => output.push_str("undef"),
        Expression::NewTarget => output.push_str("new.target"),
        Expression::Identifier(name) => {
            output.push_str("id:");
            push_limited_shape_text(output, name);
        }
        Expression::This => output.push_str("this"),
        Expression::Sent => output.push_str("sent"),
        Expression::Array(elements) => {
            output.push_str("array[");
            output.push_str(&elements.len().to_string());
            for element in elements.iter().take(8) {
                output.push('|');
                match element {
                    ArrayElement::Expression(_) => output.push('e'),
                    ArrayElement::Spread(_) => output.push('s'),
                }
                let expression = match element {
                    ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                        expression
                    }
                };
                push_materialization_shape_key(expression, output, depth - 1, budget);
            }
            output.push(']');
        }
        Expression::Object(entries) => {
            output.push_str("object{");
            output.push_str(&entries.len().to_string());
            for entry in entries.iter().take(8) {
                output.push('|');
                match entry {
                    ObjectEntry::Data { key, value } => {
                        output.push('d');
                        push_materialization_shape_key(key, output, depth - 1, budget);
                        output.push(':');
                        push_materialization_shape_key(value, output, depth - 1, budget);
                    }
                    ObjectEntry::Getter { key, getter } => {
                        output.push('g');
                        push_materialization_shape_key(key, output, depth - 1, budget);
                        output.push(':');
                        push_materialization_shape_key(getter, output, depth - 1, budget);
                    }
                    ObjectEntry::Setter { key, setter } => {
                        output.push('s');
                        push_materialization_shape_key(key, output, depth - 1, budget);
                        output.push(':');
                        push_materialization_shape_key(setter, output, depth - 1, budget);
                    }
                    ObjectEntry::Spread(expression) => {
                        output.push('*');
                        push_materialization_shape_key(expression, output, depth - 1, budget);
                    }
                }
            }
            output.push('}');
        }
        Expression::Member { object, property } => {
            output.push_str("member(");
            push_materialization_shape_key(object, output, depth - 1, budget);
            output.push('.');
            push_materialization_shape_key(property, output, depth - 1, budget);
            output.push(')');
        }
        Expression::SuperMember { property } => {
            output.push_str("super.");
            push_materialization_shape_key(property, output, depth - 1, budget);
        }
        Expression::Assign { name, value } => {
            output.push_str("assign:");
            push_limited_shape_text(output, name);
            output.push('=');
            push_materialization_shape_key(value, output, depth - 1, budget);
        }
        Expression::AssignMember {
            object,
            property,
            value,
        } => {
            output.push_str("assign-member(");
            push_materialization_shape_key(object, output, depth - 1, budget);
            output.push('.');
            push_materialization_shape_key(property, output, depth - 1, budget);
            output.push('=');
            push_materialization_shape_key(value, output, depth - 1, budget);
            output.push(')');
        }
        Expression::AssignSuperMember { property, value } => {
            output.push_str("assign-super(");
            push_materialization_shape_key(property, output, depth - 1, budget);
            output.push('=');
            push_materialization_shape_key(value, output, depth - 1, budget);
            output.push(')');
        }
        Expression::Await(expression) => {
            output.push_str("await(");
            push_materialization_shape_key(expression, output, depth - 1, budget);
            output.push(')');
        }
        Expression::EnumerateKeys(expression) => {
            output.push_str("enum-keys(");
            push_materialization_shape_key(expression, output, depth - 1, budget);
            output.push(')');
        }
        Expression::GetIterator(expression) => {
            output.push_str("get-iter(");
            push_materialization_shape_key(expression, output, depth - 1, budget);
            output.push(')');
        }
        Expression::IteratorClose(expression) => {
            output.push_str("iter-close(");
            push_materialization_shape_key(expression, output, depth - 1, budget);
            output.push(')');
        }
        Expression::Unary { op, expression } => {
            output.push_str("unary:");
            output.push_str(&format!("{op:?}"));
            output.push('(');
            push_materialization_shape_key(expression, output, depth - 1, budget);
            output.push(')');
        }
        Expression::Binary { op, left, right } => {
            output.push_str("binary:");
            output.push_str(&format!("{op:?}"));
            output.push('(');
            push_materialization_shape_key(left, output, depth - 1, budget);
            output.push(',');
            push_materialization_shape_key(right, output, depth - 1, budget);
            output.push(')');
        }
        Expression::Conditional {
            condition,
            then_expression,
            else_expression,
        } => {
            output.push_str("cond(");
            push_materialization_shape_key(condition, output, depth - 1, budget);
            output.push('?');
            push_materialization_shape_key(then_expression, output, depth - 1, budget);
            output.push(':');
            push_materialization_shape_key(else_expression, output, depth - 1, budget);
            output.push(')');
        }
        Expression::Sequence(expressions) => {
            output.push_str("seq[");
            output.push_str(&expressions.len().to_string());
            for expression in expressions.iter().take(8) {
                output.push('|');
                push_materialization_shape_key(expression, output, depth - 1, budget);
            }
            output.push(']');
        }
        Expression::Call { callee, arguments }
        | Expression::SuperCall { callee, arguments }
        | Expression::New { callee, arguments } => {
            let tag = match expression {
                Expression::Call { .. } => "call",
                Expression::SuperCall { .. } => "super-call",
                Expression::New { .. } => "new",
                _ => unreachable!("filtered by enclosing match arm"),
            };
            output.push_str(tag);
            output.push('(');
            push_materialization_shape_key(callee, output, depth - 1, budget);
            output.push_str(")[");
            output.push_str(&arguments.len().to_string());
            for argument in arguments.iter().take(8) {
                output.push('|');
                match argument {
                    CallArgument::Expression(expression) => {
                        output.push('e');
                        push_materialization_shape_key(expression, output, depth - 1, budget);
                    }
                    CallArgument::Spread(expression) => {
                        output.push('s');
                        push_materialization_shape_key(expression, output, depth - 1, budget);
                    }
                }
            }
            output.push(']');
        }
        Expression::Update { name, op, prefix } => {
            output.push_str("update:");
            output.push_str(&format!("{op:?}:"));
            output.push_str(if *prefix { "pre:" } else { "post:" });
            push_limited_shape_text(output, name);
        }
    }
}

fn materialization_shape_key(expression: &Expression) -> String {
    let mut output = String::with_capacity(256);
    let mut budget = MATERIALIZATION_SHAPE_NODE_LIMIT;
    push_materialization_shape_key(
        expression,
        &mut output,
        MATERIALIZATION_SHAPE_DEPTH_LIMIT,
        &mut budget,
    );
    output
}

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn materialize_static_expression(
        &self,
        expression: &Expression,
    ) -> Expression {
        let guard_key = expression as *const Expression as usize;
        {
            let mut active = self
                .state
                .speculation
                .static_semantics
                .materializing_expression_keys
                .borrow_mut();
            if !active.insert(guard_key) {
                return expression.clone();
            }
        }
        let _guard = MaterializationGuard {
            active: &self
                .state
                .speculation
                .static_semantics
                .materializing_expression_keys,
            key: guard_key,
        };
        let structural_key = materialization_shape_key(expression);
        let inserted = ACTIVE_MATERIALIZATION_SHAPES
            .with(|active| active.borrow_mut().insert(structural_key.clone()));
        if !inserted {
            return expression.clone();
        }
        let _structural_guard = StructuralMaterializationGuard {
            key: structural_key,
        };
        match expression {
            Expression::Identifier(name) => {
                self.materialize_identifier_expression(name, expression)
            }
            Expression::Member { object, property } => {
                self.materialize_member_expression(object, property)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.materialize_conditional_expression(condition, then_expression, else_expression)
            }
            Expression::Await(_) => {
                match self.resolve_static_await_resolution_outcome(expression) {
                    Some(StaticEvalOutcome::Value(value)) => {
                        self.materialize_static_expression(&value)
                    }
                    _ => self.materialize_recursive_expression_default(expression),
                }
            }
            Expression::Call { callee, arguments } => {
                self.materialize_call_expression(expression, callee, arguments)
            }
            _ => self.materialize_recursive_expression_default(expression),
        }
    }
}
