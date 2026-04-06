use crate::lexer::Span;
use crate::types::Type;

#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

impl Expr {
    pub fn new(kind: ExprKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Null,
    Ident(String),
    BinOp {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    UnaryOp {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<CallArg>,
    },
    Index {
        obj: Box<Expr>,
        idx: Box<Expr>,
    },
    Field {
        obj: Box<Expr>,
        field: String,
        field_span: Span,
    },
    List(Vec<Expr>),
    Map(Vec<(Expr, Expr)>),
    Lambda {
        params: Vec<Param>,
        ret_ty: Option<Type>,
        ret_ty_span: Option<Span>,
        body: Vec<Stmt>,
    },
    Interp(Vec<InterpPart>),
    Ternary {
        cond: Box<Expr>,
        then: Box<Expr>,
        else_: Box<Expr>,
    },
    Propagate(Box<Expr>),
    Spawn(Box<Expr>),
}

#[derive(Debug, Clone)]
pub enum InterpPart {
    Str(String),
    Expr(Expr),
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Option<Type>,
    pub span: Span,
    pub ty_span: Option<Span>,
}

#[derive(Debug, Clone)]
pub struct CallArg {
    pub name: Option<String>,
    pub expr: Expr,
}

#[derive(Debug, Clone, Copy)]
pub enum BinOp {
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    EqEq,
    BangEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
    DotDot,
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    Minus,
    Not,
}

#[derive(Debug, Clone)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

impl Stmt {
    pub fn new(kind: StmtKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Clone)]
pub enum StmtKind {
    Assign {
        target: String,
        target_span: Span,
        ty: Option<Type>,
        ty_span: Option<Span>,
        value: Expr,
    },
    FieldAssign {
        obj: Expr,
        field: String,
        field_span: Span,
        value: Expr,
    },
    IndexAssign {
        obj: Expr,
        idx: Expr,
        value: Expr,
    },
    Return(Option<Expr>),
    If {
        cond: Expr,
        then: Vec<Stmt>,
        elifs: Vec<(Expr, Vec<Stmt>)>,
        else_: Option<Vec<Stmt>>,
    },
    For {
        var: String,
        var_span: Span,
        iter: Expr,
        step: Option<Expr>,
        body: Vec<Stmt>,
    },
    ForKV {
        key: String,
        key_span: Span,
        val: String,
        val_span: Span,
        iter: Expr,
        body: Vec<Stmt>,
    },
    While {
        cond: Expr,
        body: Vec<Stmt>,
    },
    Break,
    Continue,
    Expr(Expr),
    Load {
        alias: String,
        alias_span: Span,
        source: String,
        source_span: Option<Span>,
    },
    FnDef(FnDef),
    ClsDef(ClsDef),
    Export(Box<Stmt>),
}

#[derive(Debug, Clone)]
pub struct FnDef {
    pub name: String,
    pub name_span: Span,
    pub params: Vec<Param>,
    pub ret_ty: Option<Type>,
    pub ret_ty_span: Option<Span>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct ClsDef {
    pub name: String,
    pub name_span: Span,
    pub fields: Vec<Param>,
}
