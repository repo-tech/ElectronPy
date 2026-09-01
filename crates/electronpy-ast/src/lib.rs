use serde::Deserialize;

/// Python AST representation converted from Python's ast module
#[derive(Debug, Clone, Deserialize)]
pub struct Module {
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum Stmt {
    #[serde(rename = "assign")]
    Assign { target: Expr, value: Expr },

    #[serde(rename = "expr")]
    Expr { value: Expr },

    #[serde(rename = "if")]
    If {
        test: Expr,
        body: Vec<Stmt>,
        orelse: Vec<Stmt>,
    },

    #[serde(rename = "while")]
    While { test: Expr, body: Vec<Stmt> },

    #[serde(rename = "for")]
    For {
        target: Expr,
        iter: Expr,
        body: Vec<Stmt>,
    },

    #[serde(rename = "funcdef")]
    FunctionDef {
        name: String,
        args: Vec<String>,
        #[serde(default)]
        arg_annotations: Vec<Option<String>>,
        body: Vec<Stmt>,
        returns: Option<String>,
    },

    #[serde(rename = "return")]
    Return { value: Option<Expr> },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum Expr {
    #[serde(rename = "name")]
    Name { id: String },

    #[serde(rename = "int")]
    Int { value: i64 },

    #[serde(rename = "float")]
    Float { value: f64 },

    #[serde(rename = "string")]
    String { value: String },

    #[serde(rename = "bool")]
    Bool { value: bool },

    #[serde(rename = "none")]
    None,

    #[serde(rename = "binary")]
    Binary {
        left: Box<Expr>,
        operator: String,
        right: Box<Expr>,
    },

    #[serde(rename = "compare")]
    Compare {
        left: Box<Expr>,
        operators: Vec<String>,
        comparators: Vec<Expr>,
    },

    #[serde(rename = "call")]
    Call {
        function: Box<Expr>,
        args: Vec<Expr>,
    },

    #[serde(rename = "list")]
    List { elements: Vec<Expr> },

    #[serde(rename = "subscript")]
    Subscript { value: Box<Expr>, index: Box<Expr> },
}

impl Expr {
    /// Get a debug representation suitable for error messages
    pub fn kind_name(&self) -> &'static str {
        match self {
            Expr::Name { .. } => "name",
            Expr::Int { .. } => "int",
            Expr::Float { .. } => "float",
            Expr::String { .. } => "string",
            Expr::Bool { .. } => "bool",
            Expr::None => "None",
            Expr::Binary { .. } => "binary operation",
            Expr::Compare { .. } => "comparison",
            Expr::Call { .. } => "function call",
            Expr::List { .. } => "list",
            Expr::Subscript { .. } => "subscript",
        }
    }
}
