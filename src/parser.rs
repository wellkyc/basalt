use std::rc::Rc;

use crate::ast::*;
use crate::error::{Error, Span};
use crate::lexer::{Keyword as K, Tok, Token, tokenize};

type Result<T> = std::result::Result<T, Error>;

pub fn parse(source: &str) -> Result<Program> {
    let mut parser = Parser::new(tokenize(source)?);
    let body = parser.block(true, &|_| false)?;
    parser.expect_eof()?;
    Ok(Program {
        body,
        procedures: parser.procedures,
        data: parser.data,
    })
}

pub fn parse_expression(source: &str) -> Result<Expr> {
    let mut parser = Parser::new(tokenize(source)?);
    parser.skip_separators();
    let expr = parser.expr()?;
    parser.skip_separators();
    parser.expect_eof()?;
    Ok(expr)
}

fn format_label(n: f64) -> String {
    if n.fract() == 0.0 {
        format!("{}", n as i64)
    } else {
        n.to_string()
    }
}

fn infix(tok: &Tok) -> Option<(BinOp, u8, u8)> {
    Some(match tok {
        Tok::Keyword(K::Xor) => (BinOp::Xor, 1, 2),
        Tok::Keyword(K::Or) => (BinOp::Or, 3, 4),
        Tok::Keyword(K::And) => (BinOp::And, 5, 6),
        Tok::Eq => (BinOp::Eq, 9, 10),
        Tok::Ne => (BinOp::Ne, 9, 10),
        Tok::Lt => (BinOp::Lt, 9, 10),
        Tok::Gt => (BinOp::Gt, 9, 10),
        Tok::Le => (BinOp::Le, 9, 10),
        Tok::Ge => (BinOp::Ge, 9, 10),
        Tok::Plus => (BinOp::Add, 11, 12),
        Tok::Minus => (BinOp::Sub, 11, 12),
        Tok::Keyword(K::Mod) => (BinOp::Mod, 13, 14),
        Tok::Backslash => (BinOp::IntDiv, 15, 16),
        Tok::Star => (BinOp::Mul, 17, 18),
        Tok::Slash => (BinOp::Div, 17, 18),
        Tok::Caret => (BinOp::Pow, 21, 22),
        _ => return None,
    })
}

const NOT_POWER: u8 = 7;
const NEG_POWER: u8 = 19;

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    depth: usize,
    procedures: Vec<Rc<Procedure>>,
    data: Vec<DataItem>,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            depth: 0,
            procedures: Vec::new(),
            data: Vec::new(),
        }
    }

    fn peek(&self) -> &Tok {
        &self.tokens[self.pos].tok
    }

    fn peek_at(&self, offset: usize) -> &Tok {
        &self.tokens[(self.pos + offset).min(self.tokens.len() - 1)].tok
    }

    fn span(&self) -> Span {
        self.tokens[self.pos].span
    }

    fn prev_span(&self) -> Span {
        self.tokens[self.pos.saturating_sub(1)].span
    }

    fn advance(&mut self) -> Token {
        let token = self.tokens[self.pos].clone();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        token
    }

    fn at(&self, keyword: K) -> bool {
        *self.peek() == Tok::Keyword(keyword)
    }

    fn at_end_of(&self, keyword: K) -> bool {
        self.at(K::End) && *self.peek_at(1) == Tok::Keyword(keyword)
    }

    fn eat(&mut self, keyword: K) -> bool {
        let found = self.at(keyword);
        if found {
            self.advance();
        }
        found
    }

    fn eat_tok(&mut self, tok: &Tok) -> bool {
        let found = self.peek() == tok;
        if found {
            self.advance();
        }
        found
    }

    fn error_here(&self, message: impl Into<String>) -> Error {
        if *self.peek() == Tok::Eof {
            Error::incomplete(message, self.span())
        } else {
            Error::syntax(message, self.span())
        }
    }

    fn expect_tok(&mut self, tok: Tok) -> Result<Span> {
        if *self.peek() == tok {
            Ok(self.advance().span)
        } else {
            Err(self.error_here(format!(
                "Expected {}, found {}",
                tok.describe(),
                self.peek().describe()
            )))
        }
    }

    fn expect(&mut self, keyword: K) -> Result<Span> {
        self.expect_tok(Tok::Keyword(keyword))
    }

    fn expect_end(&mut self, keyword: K) -> Result<()> {
        if self.at_end_of(keyword) {
            self.advance();
            self.advance();
            Ok(())
        } else {
            Err(self.error_here(format!(
                "Expected END {}, found {}",
                keyword.name(),
                self.peek().describe()
            )))
        }
    }

    fn expect_ident(&mut self) -> Result<(String, Span)> {
        match self.peek().clone() {
            Tok::Ident(name) => Ok((name, self.advance().span)),
            other => Err(self.error_here(format!("Expected a name, found {}", other.describe()))),
        }
    }

    fn expect_eof(&mut self) -> Result<()> {
        match self.peek() {
            Tok::Eof => Ok(()),
            other => Err(Error::syntax(
                format!("Unexpected {}", other.describe()),
                self.span(),
            )),
        }
    }

    fn at_stmt_end(&self) -> bool {
        matches!(
            self.peek(),
            Tok::Newline | Tok::Colon | Tok::Eof | Tok::Keyword(K::Else)
        )
    }

    fn skip_separators(&mut self) {
        while matches!(self.peek(), Tok::Newline | Tok::Colon) {
            self.advance();
        }
    }

    fn block(&mut self, top: bool, stop: &dyn Fn(&Parser) -> bool) -> Result<Block> {
        self.depth += 1;
        let result = self.block_inner(top, stop);
        self.depth -= 1;
        result
    }

    fn block_inner(&mut self, top: bool, stop: &dyn Fn(&Parser) -> bool) -> Result<Block> {
        let mut block = Block::default();
        let mut line_start = top;
        loop {
            loop {
                match self.peek() {
                    Tok::Newline => line_start = true,
                    Tok::Colon => {}
                    _ => break,
                }
                self.advance();
            }
            if *self.peek() == Tok::Eof || stop(self) {
                return Ok(block);
            }
            if line_start && let Some(label) = self.label_here() {
                let span = self.advance().span;
                if block
                    .labels
                    .insert(label.clone(), block.stmts.len())
                    .is_some()
                {
                    return Err(Error::syntax(format!("Duplicate label {label}"), span));
                }
                block.stmts.push(Stmt {
                    kind: StmtKind::Label(label),
                    span,
                });
                line_start = false;
                continue;
            }
            line_start = false;
            if self.at(K::Sub) || self.at(K::Function) {
                if self.depth != 1 {
                    return Err(Error::syntax(
                        "SUB and FUNCTION can only be defined at the top level",
                        self.span(),
                    ));
                }
                let procedure = self.procedure()?;
                self.procedures.push(Rc::new(procedure));
                continue;
            }
            block.stmts.push(self.statement()?);
            if !matches!(self.peek(), Tok::Newline | Tok::Colon | Tok::Eof) && !stop(self) {
                return Err(Error::syntax(
                    format!(
                        "Expected end of statement, found {}",
                        self.peek().describe()
                    ),
                    self.span(),
                ));
            }
        }
    }

    fn label_here(&self) -> Option<String> {
        match self.peek() {
            Tok::Number(n) => Some(format_label(*n)),
            Tok::Ident(name) if *self.peek_at(1) == Tok::Colon => Some(name.clone()),
            _ => None,
        }
    }

    fn label_ref(&mut self) -> Result<String> {
        match self.peek().clone() {
            Tok::Number(n) => {
                self.advance();
                Ok(format_label(n))
            }
            Tok::Ident(name) => {
                self.advance();
                Ok(name)
            }
            other => Err(self.error_here(format!("Expected a label, found {}", other.describe()))),
        }
    }

    fn procedure(&mut self) -> Result<Procedure> {
        let is_function = self.at(K::Function);
        let start = self.advance().span;
        let (name, name_span) = self.expect_ident()?;
        let mut params = Vec::new();
        if self.eat_tok(&Tok::LParen) {
            if *self.peek() != Tok::RParen {
                loop {
                    let (param, _) = self.expect_ident()?;
                    if *self.peek() == Tok::LParen && *self.peek_at(1) == Tok::RParen {
                        self.advance();
                        self.advance();
                    }
                    if self.eat(K::As) {
                        self.expect_ident()?;
                    }
                    if params.contains(&param) {
                        return Err(Error::syntax(
                            format!("Duplicate parameter {param}"),
                            self.prev_span(),
                        ));
                    }
                    params.push(param);
                    if !self.eat_tok(&Tok::Comma) {
                        break;
                    }
                }
            }
            self.expect_tok(Tok::RParen)?;
        }
        let end = if is_function { K::Function } else { K::Sub };
        let body = self.block(false, &move |p: &Parser| p.at_end_of(end))?;
        self.expect_end(end)?;
        Ok(Procedure {
            name,
            params,
            body,
            is_function,
            span: start.to(name_span),
        })
    }

    fn statement(&mut self) -> Result<Stmt> {
        let start = self.span();
        let kind = match self.peek().clone() {
            Tok::Keyword(keyword) => self.keyword_statement(keyword, start)?,
            Tok::Ident(_) => self.assign_or_call()?,
            other => {
                return Err(
                    self.error_here(format!("Expected a statement, found {}", other.describe()))
                );
            }
        };
        Ok(Stmt {
            kind,
            span: start.to(self.prev_span()),
        })
    }

    fn keyword_statement(&mut self, keyword: K, start: Span) -> Result<StmtKind> {
        if keyword == K::End
            && let Tok::Keyword(next @ (K::If | K::Sub | K::Function | K::Select)) =
                *self.peek_at(1)
        {
            return Err(Error::syntax(
                format!("END {} without a matching {}", next.name(), next.name()),
                start,
            ));
        }
        if matches!(
            keyword,
            K::Print | K::Input | K::If | K::For | K::While | K::Do | K::Select | K::Dim | K::Data
        ) {
            return match keyword {
                K::Print => self.print(),
                K::Input => self.input(),
                K::If => self.if_stmt(),
                K::For => self.for_stmt(),
                K::While => self.while_stmt(),
                K::Do => self.do_stmt(),
                K::Select => self.select(),
                K::Dim => self.dim(),
                _ => self.data_stmt(),
            };
        }
        self.advance();
        Ok(match keyword {
            K::Let => {
                let target = self.target()?;
                self.expect_tok(Tok::Eq)?;
                StmtKind::Assign(target, self.expr()?)
            }
            K::Exit => {
                let kind = match self.peek() {
                    Tok::Keyword(K::For) => ExitKind::For,
                    Tok::Keyword(K::Do) => ExitKind::Do,
                    Tok::Keyword(K::While) => ExitKind::While,
                    Tok::Keyword(K::Sub) => ExitKind::Sub,
                    Tok::Keyword(K::Function) => ExitKind::Function,
                    _ => {
                        return Err(
                            self.error_here("Expected FOR, DO, WHILE, SUB or FUNCTION after EXIT")
                        );
                    }
                };
                self.advance();
                StmtKind::Exit(kind)
            }
            K::Call => {
                let (name, _) = self.expect_ident()?;
                let args = if *self.peek() == Tok::LParen {
                    self.paren_args()?
                } else {
                    Vec::new()
                };
                StmtKind::Call(name, args)
            }
            K::Const => {
                let (name, _) = self.expect_ident()?;
                self.expect_tok(Tok::Eq)?;
                StmtKind::Const(name, self.expr()?)
            }
            K::Goto => StmtKind::Goto(self.label_ref()?),
            K::Gosub => StmtKind::Gosub(self.label_ref()?),
            K::Return => StmtKind::Return,
            K::Swap => {
                let a = self.target()?;
                self.expect_tok(Tok::Comma)?;
                StmtKind::Swap(a, self.target()?)
            }
            K::Randomize => StmtKind::Randomize(self.optional_expr()?),
            K::Cls => StmtKind::Cls,
            K::Color => {
                let fg = if self.at_stmt_end() || *self.peek() == Tok::Comma {
                    None
                } else {
                    Some(self.expr()?)
                };
                let bg = if self.eat_tok(&Tok::Comma) {
                    Some(self.expr()?)
                } else {
                    None
                };
                StmtKind::Color(fg, bg)
            }
            K::Locate => {
                let row = self.expr()?;
                self.expect_tok(Tok::Comma)?;
                StmtKind::Locate(row, self.expr()?)
            }
            K::Sleep => StmtKind::Sleep(self.optional_expr()?),
            K::Beep => StmtKind::Beep,
            K::Read => StmtKind::Read(self.target_list()?),
            K::Restore => StmtKind::Restore,
            K::End => StmtKind::End,
            other => {
                return Err(Error::syntax(format!("Unexpected {}", other.name()), start));
            }
        })
    }

    fn optional_expr(&mut self) -> Result<Option<Expr>> {
        if self.at_stmt_end() {
            Ok(None)
        } else {
            Ok(Some(self.expr()?))
        }
    }

    fn assign_or_call(&mut self) -> Result<StmtKind> {
        let (name, span) = self.expect_ident()?;
        if *self.peek() == Tok::LParen {
            let args = self.paren_args()?;
            if self.eat_tok(&Tok::Eq) {
                let target = Target {
                    name,
                    indices: Some(args),
                    span: span.to(self.tokens[self.pos - 2].span),
                };
                return Ok(StmtKind::Assign(target, self.expr()?));
            }
            return Ok(StmtKind::Call(name, args));
        }
        if self.eat_tok(&Tok::Eq) {
            let target = Target {
                name,
                indices: None,
                span,
            };
            return Ok(StmtKind::Assign(target, self.expr()?));
        }
        let mut args = Vec::new();
        if !self.at_stmt_end() {
            loop {
                args.push(self.expr()?);
                if !self.eat_tok(&Tok::Comma) {
                    break;
                }
            }
        }
        Ok(StmtKind::Call(name, args))
    }

    fn target(&mut self) -> Result<Target> {
        let (name, span) = self.expect_ident()?;
        if *self.peek() == Tok::LParen {
            let indices = self.paren_args()?;
            Ok(Target {
                name,
                indices: Some(indices),
                span: span.to(self.prev_span()),
            })
        } else {
            Ok(Target {
                name,
                indices: None,
                span,
            })
        }
    }

    fn target_list(&mut self) -> Result<Vec<Target>> {
        let mut targets = vec![self.target()?];
        while self.eat_tok(&Tok::Comma) {
            targets.push(self.target()?);
        }
        Ok(targets)
    }

    fn paren_args(&mut self) -> Result<Vec<Expr>> {
        self.expect_tok(Tok::LParen)?;
        let mut args = Vec::new();
        if *self.peek() != Tok::RParen {
            loop {
                args.push(self.expr()?);
                if !self.eat_tok(&Tok::Comma) {
                    break;
                }
            }
        }
        self.expect_tok(Tok::RParen)?;
        Ok(args)
    }

    fn print(&mut self) -> Result<StmtKind> {
        self.advance();
        let mut items = Vec::new();
        while !self.at_stmt_end() {
            match self.peek() {
                Tok::Semicolon => {
                    self.advance();
                    items.push(PrintItem::Semicolon);
                }
                Tok::Comma => {
                    self.advance();
                    items.push(PrintItem::Comma);
                }
                _ => items.push(PrintItem::Expr(self.expr()?)),
            }
        }
        let newline = !matches!(items.last(), Some(PrintItem::Semicolon | PrintItem::Comma));
        Ok(StmtKind::Print { items, newline })
    }

    fn input(&mut self) -> Result<StmtKind> {
        self.advance();
        let mut prompt = None;
        let mut question = true;
        if let Tok::Str(text) = self.peek().clone()
            && matches!(self.peek_at(1), Tok::Semicolon | Tok::Comma)
        {
            self.advance();
            question = *self.peek() == Tok::Semicolon;
            self.advance();
            prompt = Some(text);
        }
        Ok(StmtKind::Input {
            prompt,
            question,
            targets: self.target_list()?,
        })
    }

    fn if_stmt(&mut self) -> Result<StmtKind> {
        self.advance();
        let cond = self.expr()?;
        self.expect(K::Then)?;
        if matches!(self.peek(), Tok::Newline | Tok::Eof) {
            let stop = |p: &Parser| p.at(K::ElseIf) || p.at(K::Else) || p.at_end_of(K::If);
            let mut branches = vec![(cond, self.block(false, &stop)?)];
            let mut otherwise = None;
            loop {
                if self.eat(K::ElseIf) {
                    let cond = self.expr()?;
                    self.expect(K::Then)?;
                    branches.push((cond, self.block(false, &stop)?));
                } else if self.eat(K::Else) {
                    otherwise = Some(self.block(false, &|p| p.at_end_of(K::If))?);
                    self.expect_end(K::If)?;
                    break;
                } else {
                    self.expect_end(K::If)?;
                    break;
                }
            }
            return Ok(StmtKind::If {
                branches,
                otherwise,
            });
        }
        let then_block = self.inline_branch()?;
        let otherwise = if self.eat(K::Else) {
            Some(self.inline_branch()?)
        } else {
            None
        };
        Ok(StmtKind::If {
            branches: vec![(cond, then_block)],
            otherwise,
        })
    }

    fn inline_branch(&mut self) -> Result<Block> {
        let mut block = Block::default();
        if let Tok::Number(n) = *self.peek() {
            let span = self.advance().span;
            block.stmts.push(Stmt {
                kind: StmtKind::Goto(format_label(n)),
                span,
            });
            return Ok(block);
        }
        loop {
            block.stmts.push(self.statement()?);
            if *self.peek() != Tok::Colon {
                break;
            }
            self.advance();
            if matches!(self.peek(), Tok::Newline | Tok::Eof) || self.at(K::Else) {
                break;
            }
        }
        Ok(block)
    }

    fn for_stmt(&mut self) -> Result<StmtKind> {
        self.advance();
        let (var, _) = self.expect_ident()?;
        self.expect_tok(Tok::Eq)?;
        let start = self.expr()?;
        self.expect(K::To)?;
        let end = self.expr()?;
        let step = if self.eat(K::Step) {
            Some(self.expr()?)
        } else {
            None
        };
        let body = self.block(false, &|p| p.at(K::Next))?;
        self.expect(K::Next)?;
        if let Tok::Ident(name) = self.peek().clone() {
            if name != var {
                return Err(Error::syntax(
                    format!("NEXT {name} does not match FOR {var}"),
                    self.span(),
                ));
            }
            self.advance();
        }
        Ok(StmtKind::For {
            var,
            start,
            end,
            step,
            body,
        })
    }

    fn while_stmt(&mut self) -> Result<StmtKind> {
        self.advance();
        let cond = self.expr()?;
        let body = self.block(false, &|p| p.at(K::Wend))?;
        self.expect(K::Wend)?;
        Ok(StmtKind::While { cond, body })
    }

    fn do_stmt(&mut self) -> Result<StmtKind> {
        self.advance();
        let pre = self.loop_condition()?;
        let body = self.block(false, &|p| p.at(K::Loop))?;
        self.expect(K::Loop)?;
        let post = self.loop_condition()?;
        Ok(StmtKind::Do { pre, post, body })
    }

    fn loop_condition(&mut self) -> Result<Option<LoopCondition>> {
        let until = if self.eat(K::While) {
            false
        } else if self.eat(K::Until) {
            true
        } else {
            return Ok(None);
        };
        Ok(Some(LoopCondition {
            until,
            expr: self.expr()?,
        }))
    }

    fn select(&mut self) -> Result<StmtKind> {
        self.advance();
        self.expect(K::Case)?;
        let subject = self.expr()?;
        let mut cases = Vec::new();
        let mut otherwise = None;
        let stop = |p: &Parser| p.at(K::Case) || p.at_end_of(K::Select);
        loop {
            self.skip_separators();
            if self.at_end_of(K::Select) {
                self.advance();
                self.advance();
                break;
            }
            self.expect(K::Case)?;
            if self.eat(K::Else) {
                otherwise = Some(self.block(false, &stop)?);
                continue;
            }
            let mut tests = Vec::new();
            loop {
                if self.eat(K::Is) {
                    let op = match self.peek() {
                        Tok::Eq => BinOp::Eq,
                        Tok::Ne => BinOp::Ne,
                        Tok::Lt => BinOp::Lt,
                        Tok::Gt => BinOp::Gt,
                        Tok::Le => BinOp::Le,
                        Tok::Ge => BinOp::Ge,
                        _ => return Err(self.error_here("Expected a comparison operator after IS")),
                    };
                    self.advance();
                    tests.push(CaseTest::Is(op, self.expr()?));
                } else {
                    let value = self.expr()?;
                    if self.eat(K::To) {
                        tests.push(CaseTest::Range(value, self.expr()?));
                    } else {
                        tests.push(CaseTest::Value(value));
                    }
                }
                if !self.eat_tok(&Tok::Comma) {
                    break;
                }
            }
            cases.push((tests, self.block(false, &stop)?));
        }
        Ok(StmtKind::Select {
            subject,
            cases,
            otherwise,
        })
    }

    fn dim(&mut self) -> Result<StmtKind> {
        self.advance();
        let shared = self.eat(K::Shared);
        let mut decls = Vec::new();
        loop {
            let (name, span) = self.expect_ident()?;
            let bounds = if *self.peek() == Tok::LParen {
                Some(self.paren_args()?)
            } else {
                None
            };
            let mut string = name.ends_with('$');
            if self.eat(K::As) {
                let (ty, ty_span) = self.expect_ident()?;
                match ty.as_str() {
                    "STRING" => string = true,
                    "INTEGER" | "LONG" | "SINGLE" | "DOUBLE" => {}
                    _ => return Err(Error::syntax(format!("Unknown type {ty}"), ty_span)),
                }
            }
            decls.push(DimDecl {
                name,
                bounds,
                string,
                span,
            });
            if !self.eat_tok(&Tok::Comma) {
                break;
            }
        }
        Ok(StmtKind::Dim { shared, decls })
    }

    fn data_stmt(&mut self) -> Result<StmtKind> {
        self.advance();
        loop {
            let negative = self.eat_tok(&Tok::Minus);
            let item = match self.peek().clone() {
                Tok::Number(n) => DataItem::Num(if negative { -n } else { n }),
                Tok::Str(s) if !negative => DataItem::Str(s),
                other => {
                    return Err(self.error_here(format!(
                        "Expected a number or string in DATA, found {}",
                        other.describe()
                    )));
                }
            };
            self.advance();
            self.data.push(item);
            if !self.eat_tok(&Tok::Comma) {
                break;
            }
        }
        Ok(StmtKind::Nop)
    }

    fn expr(&mut self) -> Result<Expr> {
        self.expr_bp(0)
    }

    fn expr_bp(&mut self, min_power: u8) -> Result<Expr> {
        let mut lhs = self.prefix()?;
        while let Some((op, left, right)) = infix(self.peek()) {
            if left < min_power {
                break;
            }
            self.advance();
            let rhs = self.expr_bp(right)?;
            let span = lhs.span.to(rhs.span);
            lhs = Expr {
                kind: ExprKind::Binary(op, Box::new(lhs), Box::new(rhs)),
                span,
            };
        }
        Ok(lhs)
    }

    fn unary(&mut self, op: UnaryOp, span: Span, power: u8) -> Result<Expr> {
        let operand = self.expr_bp(power)?;
        Ok(Expr {
            span: span.to(operand.span),
            kind: ExprKind::Unary(op, Box::new(operand)),
        })
    }

    fn prefix(&mut self) -> Result<Expr> {
        if *self.peek() == Tok::Eof {
            return Err(Error::incomplete("Expected an expression", self.span()));
        }
        let token = self.advance();
        let span = token.span;
        let kind = match token.tok {
            Tok::Number(n) => ExprKind::Number(n),
            Tok::Str(s) => ExprKind::Str(s.into()),
            Tok::Keyword(K::True) => ExprKind::Bool(true),
            Tok::Keyword(K::False) => ExprKind::Bool(false),
            Tok::Ident(name) => {
                if *self.peek() == Tok::LParen {
                    let args = self.paren_args()?;
                    return Ok(Expr {
                        kind: ExprKind::Call(name, args),
                        span: span.to(self.prev_span()),
                    });
                }
                ExprKind::Var(name)
            }
            Tok::Minus => return self.unary(UnaryOp::Neg, span, NEG_POWER),
            Tok::Plus => return self.expr_bp(NEG_POWER),
            Tok::Keyword(K::Not) => return self.unary(UnaryOp::Not, span, NOT_POWER),
            Tok::LParen => {
                let inner = self.expr()?;
                self.expect_tok(Tok::RParen)?;
                return Ok(Expr {
                    kind: inner.kind,
                    span: span.to(self.prev_span()),
                });
            }
            other => {
                return Err(Error::syntax(
                    format!("Expected an expression, found {}", other.describe()),
                    span,
                ));
            }
        };
        Ok(Expr { kind, span })
    }
}
