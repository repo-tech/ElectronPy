use anyhow::{anyhow, Result};
use electronpy_ir::TypeContext;
use electronpy_types::Type;

#[derive(Default)]
pub struct TypeInference {
    context: TypeContext,
}

impl TypeInference {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn context(&self) -> &TypeContext {
        &self.context
    }

    pub fn declare(&mut self, name: String, ty: Type) {
        self.context.declare(name, ty);
    }

    pub fn infer_expr(&self, expr: &electronpy_ast::Expr) -> Result<Type> {
        match expr {
            electronpy_ast::Expr::Int { .. } => Ok(Type::Int),
            electronpy_ast::Expr::Float { .. } => Ok(Type::Float),
            electronpy_ast::Expr::String { .. } => Ok(Type::String),
            electronpy_ast::Expr::Bool { .. } => Ok(Type::Bool),
            electronpy_ast::Expr::None => Ok(Type::None),
            electronpy_ast::Expr::Name { id } => self
                .context
                .lookup(id)
                .cloned()
                .ok_or_else(|| anyhow!("undefined variable: {}", id)),
            electronpy_ast::Expr::Binary {
                left,
                operator,
                right,
            } => {
                let left_type = self.infer_expr(left)?;
                let right_type = self.infer_expr(right)?;

                self.infer_binary_op(&left_type, operator, &right_type)
            }
            electronpy_ast::Expr::Compare { .. } => Ok(Type::Bool),
            electronpy_ast::Expr::Call { function, .. } => {
                // For now, assume built-in functions have known return types
                if let electronpy_ast::Expr::Name { id } = function.as_ref() {
                    match id.as_str() {
                        "print" => Ok(Type::None),
                        "len" => Ok(Type::Int),
                        "str" => Ok(Type::String),
                        "int" => Ok(Type::Int),
                        "float" => Ok(Type::Float),
                        "bool" => Ok(Type::Bool),
                        "range" => Ok(Type::Array(Box::new(Type::Int))),
                        _ => Err(anyhow!("unknown function: {}", id)),
                    }
                } else {
                    Err(anyhow!("cannot infer type of complex function call"))
                }
            }
            electronpy_ast::Expr::List { .. } => Ok(Type::Array(Box::new(Type::Unknown))),
            electronpy_ast::Expr::Subscript { .. } => Ok(Type::Unknown),
        }
    }

    fn infer_binary_op(&self, left: &Type, op: &str, right: &Type) -> Result<Type> {
        match (left, right) {
            (Type::Int, Type::Int) => match op {
                "add" | "sub" | "mul" | "div" | "mod" => Ok(Type::Int),
                "eq" | "ne" | "lt" | "le" | "gt" | "ge" => Ok(Type::Bool),
                _ => Err(anyhow!("unknown operator: {}", op)),
            },
            (Type::Float, Type::Float) => match op {
                "add" | "sub" | "mul" | "div" => Ok(Type::Float),
                "eq" | "ne" | "lt" | "le" | "gt" | "ge" => Ok(Type::Bool),
                _ => Err(anyhow!("unknown operator: {}", op)),
            },
            (Type::Int, Type::Float) | (Type::Float, Type::Int) => match op {
                "add" | "sub" | "mul" | "div" => Ok(Type::Float),
                "eq" | "ne" | "lt" | "le" | "gt" | "ge" => Ok(Type::Bool),
                _ => Err(anyhow!("unknown operator: {}", op)),
            },
            (Type::String, Type::String) => match op {
                "add" => Ok(Type::String),
                "eq" | "ne" | "lt" | "le" | "gt" | "ge" => Ok(Type::Bool),
                _ => Err(anyhow!("unknown operator: {}", op)),
            },
            (Type::Bool, Type::Bool) => match op {
                "and" | "or" => Ok(Type::Bool),
                "eq" | "ne" => Ok(Type::Bool),
                _ => Err(anyhow!("unknown operator: {}", op)),
            },
            _ => Err(anyhow!(
                "type mismatch in binary operation: {} {} {}",
                left,
                op,
                right
            )),
        }
    }
}

pub fn infer_expr_type(context: &TypeInference, expr: &electronpy_ast::Expr) -> Result<Type> {
    context.infer_expr(expr)
}
