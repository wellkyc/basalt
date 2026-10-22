use std::collections::HashMap;
use std::rc::Rc;

use crate::error::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    IntDiv,
    Mod,
    Pow,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
    Xor,
}

#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    Number(f64),
    Str(Rc<str>),
    Bool(bool),
    Var(String),
    Call(String, Vec<Expr>),
    Unary(UnaryOp, Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone)]
pub struct Target {
    pub name: String,
    pub indices: Option<Vec<Expr>>,
    pub span: Span,
}

#[derive(Debug, Clone, Default)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub labels: HashMap<String, usize>,
}

#[derive(Debug, Clone)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum PrintItem {
    Expr(Expr),
    Semicolon,
    Comma,
}

#[derive(Debug, Clone)]
pub enum CaseTest {
    Value(Expr),
    Range(Expr, Expr),
    Is(BinOp, Expr),
}

#[derive(Debug, Clone)]
pub struct LoopCondition {
    pub until: bool,
    pub expr: Expr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitKind {
    For,
    Do,
    While,
    Sub,
    Function,
}

impl ExitKind {
    pub fn name(self) -> &'static str {
        match self {
            ExitKind::For => "FOR",
            ExitKind::Do => "DO",
            ExitKind::While => "WHILE",
            ExitKind::Sub => "SUB",
            ExitKind::Function => "FUNCTION",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DimDecl {
    pub name: String,
    pub bounds: Option<Vec<Expr>>,
    pub string: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum DataItem {
    Num(f64),
    Str(String),
}

#[derive(Debug, Clone)]
pub enum StmtKind {
    Nop,
    Label(String),
    Assign(Target, Expr),
    Print {
        items: Vec<PrintItem>,
        newline: bool,
    },
    Input {
        prompt: Option<String>,
        question: bool,
        targets: Vec<Target>,
    },
    If {
        branches: Vec<(Expr, Block)>,
        otherwise: Option<Block>,
    },
    For {
        var: String,
        start: Expr,
        end: Expr,
        step: Option<Expr>,
        body: Block,
    },
    While {
        cond: Expr,
        body: Block,
    },
    Do {
        pre: Option<LoopCondition>,
        post: Option<LoopCondition>,
        body: Block,
    },
    Select {
        subject: Expr,
        cases: Vec<(Vec<CaseTest>, Block)>,
        otherwise: Option<Block>,
    },
    Exit(ExitKind),
    Call(String, Vec<Expr>),
    Dim {
        shared: bool,
        decls: Vec<DimDecl>,
    },
    Const(String, Expr),
    Goto(String),
    Gosub(String),
    Return,
    Swap(Target, Target),
    Randomize(Option<Expr>),
    Cls,
    Color(Option<Expr>, Option<Expr>),
    Locate(Expr, Expr),
    Sleep(Option<Expr>),
    Beep,
    Read(Vec<Target>),
    Restore,
    End,
}

#[derive(Debug, Clone)]
pub struct Procedure {
    pub name: String,
    pub params: Vec<String>,
    pub body: Block,
    pub is_function: bool,
    pub span: Span,
}

#[derive(Debug, Clone, Default)]
pub struct Program {
    pub body: Block,
    pub procedures: Vec<Rc<Procedure>>,
    pub data: Vec<DataItem>,
}
