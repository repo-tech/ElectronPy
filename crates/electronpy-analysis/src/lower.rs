use anyhow::{bail, Result};
use electronpy_ir::{BinaryOp, Module, Stmt, TypeContext, Value};
use electronpy_types::Type;
use std::collections::HashMap;

pub struct Lowerer {
    type_context: TypeContext,
    function_signatures: HashMap<String, (Vec<Type>, Type)>,
}

impl Lowerer {
    pub fn new() -> Self {
        Self {
            type_context: TypeContext::new(),
            function_signatures: HashMap::new(),
        }
    }

    pub fn lower_module(&mut self, module: &electronpy_ast::Module) -> Result<Module> {
        let mut statements = Vec::new();

        for stmt in &module.body {
            if let electronpy_ast::Stmt::FunctionDef {
                name,
                args,
                arg_annotations,
                returns,
                ..
            } = stmt
            {
                let param_types: Vec<Type> = args
                    .iter()
                    .zip(arg_annotations.iter())
                    .map(|(_, annotation)| {
                        annotation
                            .as_deref()
                            .and_then(|s| self.parse_type_annotation(s))
                            .unwrap_or(Type::Unknown)
                    })
                    .collect();
                let return_type = returns
                    .as_ref()
                    .and_then(|s| self.parse_type_annotation(s))
                    .unwrap_or(Type::Unknown);
                self.function_signatures
                    .insert(name.clone(), (param_types, return_type));
            }
        }

        for stmt in &module.body {
            statements.push(self.lower_stmt(stmt)?);
        }

        Ok(Module { statements })
    }

    fn lower_stmt(&mut self, stmt: &electronpy_ast::Stmt) -> Result<Stmt> {
        match stmt {
            electronpy_ast::Stmt::Assign { target, value } => match target {
                electronpy_ast::Expr::Name { id } => {
                    let value_ir = self.lower_expr(value)?;
                    let ty = self.value_type(&value_ir)?;

                    if self.type_context.lookup(id).is_some() {
                        Ok(Stmt::Assign {
                            name: id.clone(),
                            value: value_ir,
                        })
                    } else {
                        self.type_context.declare(id.clone(), ty.clone());
                        Ok(Stmt::Let {
                            name: id.clone(),
                            ty,
                            value: value_ir,
                        })
                    }
                }
                electronpy_ast::Expr::Subscript {
                    value: container,
                    index,
                } => {
                    let electronpy_ast::Expr::Name { id } = container.as_ref() else {
                        bail!("subscript assignment target container must be a variable name");
                    };
                    let index_ir = self.lower_expr(index)?;
                    let value_ir = self.lower_expr(value)?;
                    Ok(Stmt::IndexAssign {
                        target: id.clone(),
                        index: index_ir,
                        value: value_ir,
                    })
                }
                _ => bail!("assignment target must be a variable name or subscript"),
            },

            electronpy_ast::Stmt::Expr { value } => match value {
                electronpy_ast::Expr::Call { function, args } => {
                    if let electronpy_ast::Expr::Name { id } = function.as_ref() {
                        if id == "print" {
                            let values_ir: Result<Vec<_>> =
                                args.iter().map(|a| self.lower_expr(a)).collect();
                            return Ok(Stmt::Print(values_ir?));
                        }
                    }

                    bail!("unsupported function call");
                }

                _ => {
                    bail!("unsupported expression statement");
                }
            },

            electronpy_ast::Stmt::If { test, body, orelse } => {
                let test_ir = self.lower_expr(test)?;
                let body_ir: Result<Vec<_>> = body.iter().map(|s| self.lower_stmt(s)).collect();
                let orelse_ir: Result<Vec<_>> = orelse.iter().map(|s| self.lower_stmt(s)).collect();

                Ok(Stmt::If {
                    test: test_ir,
                    body: body_ir?,
                    orelse: orelse_ir?,
                })
            }

            electronpy_ast::Stmt::While { test, body } => {
                let test_ir = self.lower_expr(test)?;
                let body_ir: Result<Vec<_>> = body.iter().map(|s| self.lower_stmt(s)).collect();

                Ok(Stmt::While {
                    test: test_ir,
                    body: body_ir?,
                })
            }

            electronpy_ast::Stmt::For { target, iter, body } => {
                let electronpy_ast::Expr::Name { id } = target else {
                    bail!("for loop target must be a variable name");
                };

                let iter_ir = self.lower_expr(iter)?;

                // Infer the element type from the iterable before lowering the body so
                // loop variables are in scope while analyzing the loop body.
                let element_type = match &iter_ir {
                    Value::Call {
                        function,
                        return_type: Type::Array(elem_type),
                        ..
                    } if function == "range" => (**elem_type).clone(),
                    _ => Type::Int,
                };
                self.type_context.declare(id.clone(), element_type.clone());

                let body_ir: Result<Vec<_>> = body.iter().map(|s| self.lower_stmt(s)).collect();

                Ok(Stmt::For {
                    target: id.clone(),
                    iter: iter_ir,
                    body: body_ir?,
                })
            }

            electronpy_ast::Stmt::FunctionDef {
                name,
                args,
                arg_annotations,
                body,
                returns,
            } => {
                let parent_context = self.type_context.clone();
                let mut function_context = TypeContext::new();
                let mut params = Vec::new();

                for (arg_name, arg_annotation) in args.iter().zip(arg_annotations.iter()) {
                    let arg_type = arg_annotation
                        .as_deref()
                        .and_then(|s| self.parse_type_annotation(s))
                        .unwrap_or(Type::Unknown);
                    function_context.declare(arg_name.clone(), arg_type.clone());
                    params.push((arg_name.clone(), arg_type));
                }

                self.type_context = function_context;
                let body_ir: Result<Vec<_>> = body.iter().map(|s| self.lower_stmt(s)).collect();
                let return_type = returns
                    .as_ref()
                    .and_then(|s| self.parse_type_annotation(s))
                    .or_else(|| self.infer_return_type_from_body(body))
                    .unwrap_or(Type::Unknown);
                self.type_context = parent_context;

                let param_types: Vec<Type> = params.iter().map(|(_, ty)| ty.clone()).collect();
                self.function_signatures
                    .insert(name.clone(), (param_types.clone(), return_type.clone()));

                Ok(Stmt::Function {
                    name: name.clone(),
                    params,
                    return_type,
                    body: body_ir?,
                })
            }

            electronpy_ast::Stmt::Return { value } => {
                let value_ir = value.as_ref().map(|v| self.lower_expr(v)).transpose()?;
                Ok(Stmt::Return(value_ir))
            }
        }
    }

    fn lower_expr(&mut self, expr: &electronpy_ast::Expr) -> Result<Value> {
        match expr {
            electronpy_ast::Expr::Int { value } => Ok(Value::Int(*value)),
            electronpy_ast::Expr::Float { value } => Ok(Value::Float(*value)),
            electronpy_ast::Expr::String { value } => Ok(Value::String(value.clone())),
            electronpy_ast::Expr::Bool { value } => Ok(Value::Bool(*value)),
            electronpy_ast::Expr::Name { id } => Ok(Value::Name(id.clone())),
            electronpy_ast::Expr::Binary {
                left,
                operator,
                right,
            } => {
                let left_ir = self.lower_expr(left)?;
                let right_ir = self.lower_expr(right)?;
                let left_type = self.value_type(&left_ir)?;
                let right_type = self.value_type(&right_ir)?;

                let op = self.parse_binary_op(operator)?;
                let result_type = self.infer_binary_result_type(&left_type, &op, &right_type)?;

                Ok(Value::Binary {
                    left: Box::new(left_ir),
                    op,
                    right: Box::new(right_ir),
                    ty: result_type,
                })
            }
            electronpy_ast::Expr::Compare {
                left,
                operators,
                comparators,
            } => {
                // For now, only support single comparisons
                if operators.len() != 1 || comparators.len() != 1 {
                    bail!("chained comparisons not yet supported");
                }

                let left_ir = self.lower_expr(left)?;
                let right_ir = self.lower_expr(&comparators[0])?;

                let op = self.parse_compare_op(&operators[0])?;

                Ok(Value::Binary {
                    left: Box::new(left_ir),
                    op,
                    right: Box::new(right_ir),
                    ty: Type::Bool,
                })
            }
            electronpy_ast::Expr::Call { function, args } => {
                if let electronpy_ast::Expr::Name { id } = function.as_ref() {
                    let args_ir: Result<Vec<_>> = args.iter().map(|a| self.lower_expr(a)).collect();
                    let args_ir = args_ir?;
                    let return_type = self.infer_call_return_type(id, &args_ir)?;

                    Ok(Value::Call {
                        function: id.clone(),
                        args: args_ir,
                        return_type,
                    })
                } else {
                    bail!("only direct function calls are supported")
                }
            }
            electronpy_ast::Expr::List { elements } => {
                let elements_ir: Result<Vec<_>> =
                    elements.iter().map(|e| self.lower_expr(e)).collect();
                let elements_ir = elements_ir?;

                let element_type = if let Some(first) = elements_ir.first() {
                    self.value_type(first)?
                } else {
                    Type::Unknown
                };

                Ok(Value::List {
                    elements: elements_ir,
                    element_type,
                })
            }
            electronpy_ast::Expr::Subscript { value, index } => {
                let container_ir = self.lower_expr(value)?;
                let index_ir = self.lower_expr(index)?;

                // Infer element type from the container
                let element_type = match self.value_type(&container_ir)? {
                    Type::Array(inner) => *inner,
                    _ => Type::Unknown,
                };

                Ok(Value::Index {
                    container: Box::new(container_ir),
                    index: Box::new(index_ir),
                    element_type,
                })
            }
            electronpy_ast::Expr::None => {
                bail!(
                    "None is not in the compilable subset. \
                     ElectronPy targets statically-typed code; \
                     use typed functions and avoid None values."
                )
            }
        }
    }

    fn parse_binary_op(&self, op: &str) -> Result<BinaryOp> {
        Ok(match op {
            "add" => BinaryOp::Add,
            "sub" => BinaryOp::Sub,
            "mul" => BinaryOp::Mul,
            "div" => BinaryOp::Div,
            "mod" => BinaryOp::Mod,
            "eq" => BinaryOp::Eq,
            "ne" => BinaryOp::NotEq,
            "lt" => BinaryOp::Lt,
            "le" => BinaryOp::LtEq,
            "gt" => BinaryOp::Gt,
            "ge" => BinaryOp::GtEq,
            _ => bail!("unsupported binary operator: {}", op),
        })
    }

    fn parse_compare_op(&self, op: &str) -> Result<BinaryOp> {
        Ok(match op {
            "eq" | "==" => BinaryOp::Eq,
            "ne" | "!=" => BinaryOp::NotEq,
            "lt" | "<" => BinaryOp::Lt,
            "le" | "<=" => BinaryOp::LtEq,
            "gt" | ">" => BinaryOp::Gt,
            "ge" | ">=" => BinaryOp::GtEq,
            _ => bail!("unsupported comparison operator: {}", op),
        })
    }

    fn value_type(&self, value: &Value) -> Result<Type> {
        match value {
            Value::Int(_) => Ok(Type::Int),
            Value::Float(_) => Ok(Type::Float),
            Value::String(_) => Ok(Type::String),
            Value::Bool(_) => Ok(Type::Bool),
            Value::Name(id) => Ok(self
                .type_context
                .lookup(id)
                .cloned()
                .unwrap_or(Type::Unknown)),
            Value::Binary { ty, .. } => Ok(ty.clone()),
            Value::Call { return_type, .. } => Ok(return_type.clone()),
            Value::List { element_type, .. } => Ok(Type::Array(Box::new(element_type.clone()))),
            Value::Index { element_type, .. } => Ok(element_type.clone()),
        }
    }

    fn infer_binary_result_type(&self, left: &Type, op: &BinaryOp, right: &Type) -> Result<Type> {
        if matches!(left, Type::Unknown) || matches!(right, Type::Unknown) {
            return Ok(Type::Unknown);
        }

        match (left, right, op) {
            (Type::Int, Type::Int, BinaryOp::Add) => Ok(Type::Int),
            (Type::Int, Type::Int, BinaryOp::Sub) => Ok(Type::Int),
            (Type::Int, Type::Int, BinaryOp::Mul) => Ok(Type::Int),
            (Type::Int, Type::Int, BinaryOp::Div) => Ok(Type::Float), // Python 3: / returns float
            (Type::Int, Type::Int, BinaryOp::Mod) => Ok(Type::Int),
            (Type::Int, Type::Int, BinaryOp::Eq) => Ok(Type::Bool),
            (Type::Int, Type::Int, BinaryOp::NotEq) => Ok(Type::Bool),
            (Type::Int, Type::Int, BinaryOp::Lt) => Ok(Type::Bool),
            (Type::Int, Type::Int, BinaryOp::LtEq) => Ok(Type::Bool),
            (Type::Int, Type::Int, BinaryOp::Gt) => Ok(Type::Bool),
            (Type::Int, Type::Int, BinaryOp::GtEq) => Ok(Type::Bool),
            (Type::Float, Type::Float, BinaryOp::Add) => Ok(Type::Float),
            (Type::Float, Type::Float, BinaryOp::Sub) => Ok(Type::Float),
            (Type::Float, Type::Float, BinaryOp::Mul) => Ok(Type::Float),
            (Type::Float, Type::Float, BinaryOp::Div) => Ok(Type::Float),
            (Type::String, Type::String, BinaryOp::Add) => Ok(Type::String),
            (Type::String, Type::String, BinaryOp::Eq) => Ok(Type::Bool),
            _ => bail!("unsupported operation: {} {} {}", left, op.symbol(), right),
        }
    }

    fn infer_call_return_type(&self, function: &str, _args: &[Value]) -> Result<Type> {
        if let Some((_, return_type)) = self.function_signatures.get(function) {
            return Ok(return_type.clone());
        }

        Ok(match function {
            "print" => Type::None,
            "len" => Type::Int,
            "str" => Type::String,
            "int" => Type::Int,
            "float" => Type::Float,
            "bool" => Type::Bool,
            "range" => Type::Array(Box::new(Type::Int)),
            _ => bail!("unknown function: {}", function),
        })
    }

    fn infer_return_type_from_body(&mut self, body: &[electronpy_ast::Stmt]) -> Option<Type> {
        let mut inferred: Option<Type> = None;

        for stmt in body {
            match stmt {
                electronpy_ast::Stmt::Return { value } => {
                    let ty = match value {
                        Some(value) => {
                            let lowered = self.lower_expr(value).ok()?;
                            self.value_type(&lowered).ok()?
                        }
                        None => Type::None,
                    };
                    inferred = Some(match inferred {
                        Some(existing) => Self::merge_return_types(&existing, &ty),
                        None => ty,
                    });
                }
                electronpy_ast::Stmt::If { body, orelse, .. } => {
                    let if_ty = self.infer_return_type_from_body(body);
                    let else_ty = self.infer_return_type_from_body(orelse);
                    let merged = match (if_ty, else_ty) {
                        (Some(a), Some(b)) => Some(Self::merge_return_types(&a, &b)),
                        (Some(a), None) => Some(a),
                        (None, Some(b)) => Some(b),
                        (None, None) => None,
                    };
                    if let Some(ty) = merged {
                        inferred = Some(match inferred {
                            Some(existing) => Self::merge_return_types(&existing, &ty),
                            None => ty,
                        });
                    }
                }
                electronpy_ast::Stmt::While { body, .. }
                | electronpy_ast::Stmt::For { body, .. } => {
                    if let Some(ty) = self.infer_return_type_from_body(body) {
                        inferred = Some(match inferred {
                            Some(existing) => Self::merge_return_types(&existing, &ty),
                            None => ty,
                        });
                    }
                }
                _ => {}
            }
        }

        inferred
    }

    fn merge_return_types(left: &Type, right: &Type) -> Type {
        if left == right {
            left.clone()
        } else if matches!(left, Type::Unknown) {
            right.clone()
        } else if matches!(right, Type::Unknown) {
            left.clone()
        } else {
            Type::Unknown
        }
    }

    fn parse_type_annotation(&self, annotation: &str) -> Option<Type> {
        let annotation = annotation.trim();
        match annotation {
            "int" | "Integer" => Some(Type::Int),
            "float" | "Float" => Some(Type::Float),
            "bool" | "Boolean" => Some(Type::Bool),
            "str" | "string" => Some(Type::String),
            "None" | "none" => Some(Type::None),
            _ => None,
        }
    }
}

pub fn lower_module(module: &electronpy_ast::Module) -> Result<Module> {
    let mut lowerer = Lowerer::new();
    lowerer.lower_module(module)
}
