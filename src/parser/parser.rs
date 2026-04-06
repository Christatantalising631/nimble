use super::ast::*;
use crate::error::ParseError;
use crate::lexer::{Lexer, Span, Token};
use crate::types::Type;

// ── Parser ────────────────────────────────────────────────────────────────────

pub struct Parser {
    tokens: Vec<(Token, Span)>,
    current: usize,
    errors: Vec<ParseError>,
}

impl Parser {
    pub fn new(tokens: Vec<(Token, Span)>) -> Self {
        // Guarantee a final EOF so every peek() is always in-bounds.
        let mut tokens = tokens;
        if !matches!(tokens.last(), Some((Token::Eof, _))) {
            let span = tokens.last().map(|(_, s)| *s).unwrap_or_default();
            tokens.push((Token::Eof, span));
        }
        Self {
            tokens,
            current: 0,
            errors: Vec::new(),
        }
    }

    // ── Navigation ────────────────────────────────────────────────────────────

    #[inline]
    fn peek(&self) -> &Token {
        // Safe: constructor guarantees EOF at end; clamp to last token.
        &self.tokens[self.current.min(self.tokens.len() - 1)].0
    }

    #[inline]
    fn peek_span(&self) -> Span {
        self.tokens[self.current.min(self.tokens.len() - 1)].1
    }

    #[inline]
    fn previous_span(&self) -> Span {
        if self.current == 0 {
            self.peek_span()
        } else {
            self.tokens[(self.current - 1).min(self.tokens.len() - 1)].1
        }
    }

    fn peek_nth(&self, n: usize) -> &Token {
        self.tokens
            .get(self.current + n)
            .map(|(t, _)| t)
            .unwrap_or(&Token::Eof)
    }

    /// Consume and return the current token. Never panics.
    fn advance(&mut self) -> Token {
        let idx = self.current.min(self.tokens.len() - 1);
        let tok = self.tokens[idx].0.clone();
        if !self.is_at_end() {
            self.current += 1;
        }
        tok
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek(), Token::Eof)
    }

    fn check(&self, token: &Token) -> bool {
        !self.is_at_end() && self.peek() == token
    }

    fn match_token(&mut self, token: Token) -> bool {
        if self.check(&token) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn consume(&mut self, token: Token, ctx: &str) -> Result<Span, ParseError> {
        if self.check(&token) {
            let span = self.peek_span();
            self.advance();
            Ok(span)
        } else {
            if let Some(delimiter) = Self::delimiter_name(&token) {
                let insertion = if matches!(self.peek(), Token::Eof) {
                    Span::point(self.previous_span().end())
                } else {
                    Span::point(self.peek_span().start)
                };
                Err(ParseError::missing_delimiter(
                    delimiter,
                    insertion,
                    Some(ctx.into()),
                ))
            } else {
                Err(ParseError::unexpected_token(
                    Self::expected_name(&token),
                    Self::actual_name(self.peek()),
                    self.peek_span(),
                ))
            }
        }
    }

    fn consume_ident(&mut self, ctx: &str) -> Result<(String, Span), ParseError> {
        match self.peek().clone() {
            Token::Ident(name) => {
                let span = self.peek_span();
                self.advance();
                Ok((name, span))
            }
            _ => Err(ParseError::unexpected_token(
                format!("an identifier ({ctx})"),
                Self::actual_name(self.peek()),
                self.peek_span(),
            )),
        }
    }

    fn consume_str_literal(&mut self, ctx: &str) -> Result<(String, Span), ParseError> {
        match self.peek().clone() {
            Token::Str(s) => {
                let span = self.peek_span();
                self.advance();
                Ok((s, span))
            }
            _ => Err(ParseError::unexpected_token(
                format!("a string literal ({ctx})"),
                Self::actual_name(self.peek()),
                self.peek_span(),
            )),
        }
    }

    fn delimiter_name(token: &Token) -> Option<&'static str> {
        match token {
            Token::RParen => Some(")"),
            Token::RBracket => Some("]"),
            Token::RBrace => Some("}"),
            Token::Colon => Some(":"),
            Token::Newline => Some("newline"),
            _ => None,
        }
    }

    fn expected_name(token: &Token) -> String {
        match token {
            Token::Indent => "an indented block".into(),
            Token::Dedent => "a dedent".into(),
            Token::LParen => "`(`".into(),
            Token::RParen => "`)`".into(),
            Token::LBracket => "`[`".into(),
            Token::RBracket => "`]`".into(),
            Token::LBrace => "`{`".into(),
            Token::RBrace => "`}`".into(),
            Token::Colon => "`:`".into(),
            Token::Comma => "`,`".into(),
            Token::Arrow => "`->`".into(),
            Token::Assign => "`=`".into(),
            Token::Newline => "a newline".into(),
            Token::In => "`in`".into(),
            Token::Else => "`else`".into(),
            other => format!("{other:?}"),
        }
    }

    fn actual_name(token: &Token) -> String {
        match token {
            Token::Ident(_) => "an identifier".into(),
            Token::Str(_) => "a string literal".into(),
            Token::Int(_) => "an integer literal".into(),
            Token::Float(_) => "a float literal".into(),
            Token::Bool(_) => "a boolean literal".into(),
            Token::Eof => "the end of input".into(),
            other => Self::expected_name(other),
        }
    }

    fn consume_newlines(&mut self) {
        while self.match_token(Token::Newline) {}
    }

    // ── Error recovery ────────────────────────────────────────────────────────

    fn record(&mut self, err: ParseError) {
        self.errors.push(err);
    }

    /// Skip tokens until we reach a safe statement boundary.
    fn synchronize(&mut self) {
        while !self.is_at_end() {
            match self.peek() {
                Token::Newline => {
                    self.advance();
                    return;
                }
                Token::Dedent => return,
                Token::Fn
                | Token::Cls
                | Token::If
                | Token::While
                | Token::For
                | Token::Return
                | Token::Load
                | Token::Export => return,
                _ => {
                    self.advance();
                }
            }
        }
    }

    // ── Public API ────────────────────────────────────────────────────────────

    /// Parse all statements, collecting recoverable errors. Returns `Err` only
    /// when at least one error was recorded; successfully parsed statements are
    /// always included in the `Ok` branch too via `parse_partial`.
    pub fn parse(&mut self) -> Result<Vec<Stmt>, Vec<ParseError>> {
        let (stmts, errors) = self.parse_partial();
        if errors.is_empty() {
            Ok(stmts)
        } else {
            Err(errors)
        }
    }

    /// Always returns whatever was successfully parsed alongside every error.
    pub fn parse_partial(&mut self) -> (Vec<Stmt>, Vec<ParseError>) {
        let mut stmts = Vec::new();
        while !self.is_at_end() {
            if self.match_token(Token::Newline) {
                continue;
            }
            match self.parse_statement() {
                Ok(stmt) => stmts.push(stmt),
                Err(err) => {
                    self.record(err);
                    self.synchronize();
                }
            }
        }
        (stmts, std::mem::take(&mut self.errors))
    }

    // ── Statements ────────────────────────────────────────────────────────────

    fn parse_statement(&mut self) -> Result<Stmt, ParseError> {
        if self.match_token(Token::Export) {
            let export_span = self.previous_span();
            let inner = self.parse_statement()?;
            let span = export_span.join(inner.span);
            return Ok(Stmt::new(StmtKind::Export(Box::new(inner)), span));
        }
        if self.match_token(Token::Fn) {
            return self.parse_fn_def();
        }
        if self.match_token(Token::Cls) {
            return self.parse_cls_def();
        }
        if self.match_token(Token::If) {
            return self.parse_if();
        }
        if self.match_token(Token::While) {
            return self.parse_while();
        }
        if self.match_token(Token::For) {
            return self.parse_for();
        }
        if self.match_token(Token::Load) {
            return self.parse_load();
        }
        if self.match_token(Token::Return) {
            return self.parse_return();
        }
        if self.match_token(Token::Break) {
            let span = self.previous_span();
            self.expect_stmt_end("after break")?;
            return Ok(Stmt::new(StmtKind::Break, span));
        }
        if self.match_token(Token::Continue) {
            let span = self.previous_span();
            self.expect_stmt_end("after continue")?;
            return Ok(Stmt::new(StmtKind::Continue, span));
        }
        self.parse_assignment_or_expr()
    }

    fn parse_return(&mut self) -> Result<Stmt, ParseError> {
        let return_span = self.previous_span();
        let has_value =
            !self.is_at_end() && !self.check(&Token::Newline) && !self.check(&Token::Dedent);
        let val = if has_value {
            Some(self.parse_expr()?)
        } else {
            None
        };
        let span = val
            .as_ref()
            .map(|expr| return_span.join(expr.span))
            .unwrap_or(return_span);
        self.expect_stmt_end("after return")?;
        Ok(Stmt::new(StmtKind::Return(val), span))
    }

    /// Consume a newline that terminates a statement, unless we're already at a
    /// dedent or EOF (both are also valid statement terminators).
    fn expect_stmt_end(&mut self, ctx: &str) -> Result<(), ParseError> {
        if self.is_at_end() || self.check(&Token::Dedent) {
            return Ok(());
        }
        self.consume(Token::Newline, &format!("expected newline {}", ctx))?;
        Ok(())
    }

    fn parse_assignment_or_expr(&mut self) -> Result<Stmt, ParseError> {
        let expr = self.parse_expr()?;
        let lhs_span = expr.span;

        // Detect assignment operator (plain or compound).
        let compound: Option<Option<BinOp>> = if self.match_token(Token::Assign) {
            Some(None)
        } else if self.match_token(Token::PlusEq) {
            Some(Some(BinOp::Plus))
        } else if self.match_token(Token::MinusEq) {
            Some(Some(BinOp::Minus))
        } else if self.match_token(Token::StarEq) {
            Some(Some(BinOp::Star))
        } else if self.match_token(Token::SlashEq) {
            Some(Some(BinOp::Slash))
        } else {
            None
        };

        if let Some(op) = compound {
            let rhs = self.parse_expr()?;
            let rhs_span = rhs.span;
            let value = match op {
                Some(bin) => Expr::new(
                    ExprKind::BinOp {
                        op: bin,
                        left: Box::new(expr.clone()),
                        right: Box::new(rhs),
                    },
                    expr.span.join(rhs_span),
                ),
                None => rhs,
            };
            self.expect_stmt_end("after assignment")?;
            let value_span = value.span;
            return match expr.kind {
                ExprKind::Ident(name) => Ok(Stmt::new(
                    StmtKind::Assign {
                        target: name,
                        target_span: lhs_span,
                        ty: None,
                        ty_span: None,
                        value,
                    },
                    lhs_span.join(value_span),
                )),
                ExprKind::Field {
                    obj,
                    field,
                    field_span,
                } => Ok(Stmt::new(
                    StmtKind::FieldAssign {
                        obj: *obj,
                        field,
                        field_span,
                        value,
                    },
                    lhs_span.join(value_span),
                )),
                ExprKind::Index { obj, idx } => Ok(Stmt::new(
                    StmtKind::IndexAssign {
                        obj: *obj,
                        idx: *idx,
                        value,
                    },
                    lhs_span.join(value_span),
                )),
                _ => Err(ParseError::new("invalid assignment target", lhs_span)),
            };
        }

        self.finish_expr_stmt(expr)
    }

    fn finish_expr_stmt(&mut self, expr: Expr) -> Result<Stmt, ParseError> {
        // Typed assignment: `name Type = value`
        if let ExprKind::Ident(name) = &expr.kind {
            let checkpoint = self.current;
            if let Some(ty) = self.try_parse_type()? {
                if self.match_token(Token::Assign) {
                    let val = self.parse_expr()?;
                    self.expect_stmt_end("after typed assignment")?;
                    let val_span = val.span;
                    return Ok(Stmt::new(
                        StmtKind::Assign {
                            target: name.clone(),
                            target_span: expr.span,
                            ty: Some(ty),
                            ty_span: None,
                            value: val,
                        },
                        expr.span.join(val_span),
                    ));
                }
                // Not a typed assignment — silently roll back.
                self.current = checkpoint;
            }
        }
        self.expect_stmt_end("after expression")?;
        Ok(Stmt::new(StmtKind::Expr(expr.clone()), expr.span))
    }

    // ── Expressions ───────────────────────────────────────────────────────────

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        let expr = self.parse_prec(Precedence::Lowest)?;

        // Postfix ternary: `value if cond else alt`
        if self.match_token(Token::If) {
            let cond = self.parse_expr()?;
            self.consume(Token::Else, "expected 'else' in conditional expression")?;
            let alt = self.parse_expr()?;
            let span = expr.span.join(alt.span);
            return Ok(Expr::new(
                ExprKind::Ternary {
                    cond: Box::new(cond),
                    then: Box::new(expr),
                    else_: Box::new(alt),
                },
                span,
            ));
        }
        Ok(expr)
    }

    fn parse_prec(&mut self, min_prec: Precedence) -> Result<Expr, ParseError> {
        let mut left = self.parse_prefix()?;
        loop {
            let prec = Precedence::of(self.peek());
            if prec <= min_prec {
                break;
            }
            left = self.parse_infix(left)?;
        }
        Ok(left)
    }

    fn parse_prefix(&mut self) -> Result<Expr, ParseError> {
        let span = self.peek_span();
        let tok = self.advance();
        match tok {
            Token::Int(n) => Ok(Expr::new(ExprKind::Int(n), span)),
            Token::Float(n) => Ok(Expr::new(ExprKind::Float(n), span)),
            Token::Bool(b) => Ok(Expr::new(ExprKind::Bool(b), span)),
            Token::Null => Ok(Expr::new(ExprKind::Null, span)),
            Token::Ident(s) => Ok(Expr::new(ExprKind::Ident(s), span)),
            Token::Error => Ok(Expr::new(ExprKind::Ident("error".into()), span)),

            Token::Str(s) => self
                .parse_interpolated_string(&s, span)
                .map_err(|err| ParseError::new(err.message, span)),

            Token::Minus => {
                let expr = self.parse_prec(Precedence::Prefix)?;
                Ok(Expr::new(
                    ExprKind::UnaryOp {
                        op: UnaryOp::Minus,
                        expr: Box::new(expr.clone()),
                    },
                    span.join(expr.span),
                ))
            }
            Token::Not => {
                let expr = self.parse_prec(Precedence::Prefix)?;
                Ok(Expr::new(
                    ExprKind::UnaryOp {
                        op: UnaryOp::Not,
                        expr: Box::new(expr.clone()),
                    },
                    span.join(expr.span),
                ))
            }

            Token::LParen => {
                let mut e = self.parse_expr()?;
                self.consume(Token::RParen, "expected ')' to close group")?;
                e.span = span.join(self.previous_span());
                Ok(e)
            }
            Token::LBracket => self.parse_list(span),
            Token::LBrace => self.parse_map(span),
            Token::Fn => self.parse_lambda(),

            Token::Spawn => {
                let expr = self.parse_expr()?;
                Ok(Expr::new(
                    ExprKind::Spawn(Box::new(expr.clone())),
                    span.join(expr.span),
                ))
            }

            other => Err(ParseError::unexpected_token(
                "an expression",
                Self::actual_name(&other),
                span,
            )),
        }
    }

    fn parse_list(&mut self, open_span: Span) -> Result<Expr, ParseError> {
        let mut items = Vec::new();
        while !self.check(&Token::RBracket) && !self.is_at_end() {
            items.push(self.parse_expr()?);
            if !self.match_token(Token::Comma) {
                break;
            }
            // trailing comma: fall through to check `]`
        }
        self.consume(Token::RBracket, "expected ']' to close list")?;
        Ok(Expr::new(
            ExprKind::List(items),
            open_span.join(self.previous_span()),
        ))
    }

    fn parse_map(&mut self, open_span: Span) -> Result<Expr, ParseError> {
        let mut pairs = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            let key = self.parse_expr()?;
            self.consume(Token::Colon, "expected ':' in map literal")?;
            let val = self.parse_expr()?;
            pairs.push((key, val));
            if !self.match_token(Token::Comma) {
                break;
            }
        }
        self.consume(Token::RBrace, "expected '}' to close map")?;
        Ok(Expr::new(
            ExprKind::Map(pairs),
            open_span.join(self.previous_span()),
        ))
    }

    fn parse_infix(&mut self, left: Expr) -> Result<Expr, ParseError> {
        let span = self.peek_span();
        let tok = self.advance();

        match &tok {
            // Binary operators
            Token::Plus
            | Token::Minus
            | Token::Star
            | Token::Slash
            | Token::Percent
            | Token::EqEq
            | Token::BangEq
            | Token::Lt
            | Token::Gt
            | Token::LtEq
            | Token::GtEq
            | Token::And
            | Token::Or
            | Token::DotDot => {
                let op = BinOp::from_token(&tok)
                    .ok_or_else(|| ParseError::new("invalid infix operator", span))?;
                let prec = Precedence::of(&tok);
                let right = self.parse_prec(prec)?;
                let span = left.span.join(right.span);
                Ok(Expr::new(
                    ExprKind::BinOp {
                        op,
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                    span,
                ))
            }

            // Call
            Token::LParen => {
                let args = self.parse_call_args()?;
                self.consume(Token::RParen, "expected ')' to close call")?;
                let expr_span = left.span.join(self.previous_span());
                Ok(Expr::new(
                    ExprKind::Call {
                        callee: Box::new(left),
                        args,
                    },
                    expr_span,
                ))
            }

            // Field access
            Token::Dot => {
                let (field, field_span) = self.consume_ident("expected field name after '.'")?;
                let expr_span = left.span.join(field_span);
                Ok(Expr::new(
                    ExprKind::Field {
                        obj: Box::new(left),
                        field,
                        field_span,
                    },
                    expr_span,
                ))
            }

            // Index
            Token::LBracket => {
                let idx = self.parse_expr()?;
                self.consume(Token::RBracket, "expected ']' to close index")?;
                let expr_span = left.span.join(self.previous_span());
                Ok(Expr::new(
                    ExprKind::Index {
                        obj: Box::new(left),
                        idx: Box::new(idx),
                    },
                    expr_span,
                ))
            }

            // Error propagation
            Token::Question => Ok(Expr::new(
                ExprKind::Propagate(Box::new(left.clone())),
                left.span.join(span),
            )),

            other => Err(ParseError::unexpected_token(
                "a valid infix operator",
                Self::actual_name(other),
                span,
            )),
        }
    }

    fn parse_call_args(&mut self) -> Result<Vec<CallArg>, ParseError> {
        let mut args = Vec::new();
        while !self.check(&Token::RParen) && !self.is_at_end() {
            // Named argument: `name = expr`
            let arg = if matches!(self.peek(), Token::Ident(_))
                && matches!(self.peek_nth(1), Token::Assign)
            {
                let (name, _) = self.consume_ident("expected argument name")?;
                self.advance(); // consume `=`
                CallArg {
                    name: Some(name),
                    expr: self.parse_expr()?,
                }
            } else {
                CallArg {
                    name: None,
                    expr: self.parse_expr()?,
                }
            };
            args.push(arg);
            if !self.match_token(Token::Comma) {
                break;
            }
        }
        Ok(args)
    }

    // ── Functions & Lambdas ───────────────────────────────────────────────────

    fn parse_fn_def(&mut self) -> Result<Stmt, ParseError> {
        let fn_span = self.previous_span();
        let (name, name_span) = self.consume_ident("expected function name")?;
        self.consume(Token::LParen, "expected '(' after function name")?;
        let params = self.parse_param_list()?;
        self.consume(Token::RParen, "expected ')' after parameters")?;
        let ret_ty = self.parse_ret_ty()?;

        let body = if self.match_token(Token::Assign) {
            // Short form: `fn f() = expr`
            let expr = self.parse_expr()?;
            self.expect_stmt_end("after single-expression function")?;
            vec![Stmt::new(StmtKind::Return(Some(expr.clone())), expr.span)]
        } else {
            self.consume(Token::Colon, "expected ':' before function body")?;
            self.consume_newlines();
            self.parse_block()?
        };

        let end_span = body.last().map(|stmt| stmt.span).unwrap_or(name_span);
        Ok(Stmt::new(
            StmtKind::FnDef(FnDef {
                name,
                name_span,
                params,
                ret_ty,
                ret_ty_span: None,
                body,
            }),
            fn_span.join(end_span),
        ))
    }

    fn parse_lambda(&mut self) -> Result<Expr, ParseError> {
        let fn_span = self.previous_span();
        self.consume(Token::LParen, "expected '(' for lambda parameters")?;
        let params = self.parse_param_list()?;
        self.consume(Token::RParen, "expected ')' after lambda parameters")?;
        let ret_ty = self.parse_ret_ty()?;

        let body = if self.match_token(Token::Assign) {
            let expr = self.parse_expr()?;
            vec![Stmt::new(StmtKind::Return(Some(expr.clone())), expr.span)]
        } else {
            self.consume(Token::Colon, "expected ':' before lambda body")?;
            self.consume_newlines();
            self.parse_block()?
        };

        let end_span = body.last().map(|stmt| stmt.span).unwrap_or(fn_span);
        Ok(Expr::new(
            ExprKind::Lambda {
                params,
                ret_ty,
                ret_ty_span: None,
                body,
            },
            fn_span.join(end_span),
        ))
    }

    fn parse_param_list(&mut self) -> Result<Vec<Param>, ParseError> {
        let mut params = Vec::new();
        while !self.check(&Token::RParen) && !self.is_at_end() {
            let (name, span) = self.consume_ident("expected parameter name")?;
            let ty = self.try_parse_type()?;
            params.push(Param {
                name,
                ty,
                span,
                ty_span: None,
            });
            if !self.match_token(Token::Comma) {
                break;
            }
        }
        Ok(params)
    }

    fn parse_ret_ty(&mut self) -> Result<Option<Type>, ParseError> {
        if self.match_token(Token::Arrow) {
            Ok(Some(self.parse_type()?))
        } else {
            Ok(None)
        }
    }

    // ── Classes ───────────────────────────────────────────────────────────────

    fn parse_cls_def(&mut self) -> Result<Stmt, ParseError> {
        let cls_span = self.previous_span();
        let (name, name_span) = self.consume_ident("expected class name")?;
        self.consume(Token::Colon, "expected ':' after class name")?;
        self.consume_newlines();
        self.consume(Token::Indent, "expected indented block for class body")?;

        let mut fields = Vec::new();
        while !self.check(&Token::Dedent) && !self.is_at_end() {
            if self.match_token(Token::Newline) {
                continue;
            }
            let (field_name, field_span) = self.consume_ident("expected field name")?;
            let ty = self.try_parse_type()?.ok_or_else(|| {
                ParseError::new(
                    format!("field '{}' requires a type annotation", field_name),
                    self.peek_span(),
                )
            })?;
            fields.push(Param {
                name: field_name,
                ty: Some(ty),
                span: field_span,
                ty_span: None,
            });
            if !self.check(&Token::Dedent) {
                self.expect_stmt_end("after class field")?;
            }
        }
        self.consume(Token::Dedent, "expected dedent after class body")?;
        let end_span = fields.last().map(|param| param.span).unwrap_or(name_span);
        Ok(Stmt::new(
            StmtKind::ClsDef(ClsDef {
                name,
                name_span,
                fields,
            }),
            cls_span.join(end_span),
        ))
    }

    // ── Blocks ────────────────────────────────────────────────────────────────

    fn parse_block(&mut self) -> Result<Vec<Stmt>, ParseError> {
        self.consume(Token::Indent, "expected indented block")?;
        let mut stmts = Vec::new();

        while !self.check(&Token::Dedent) && !self.is_at_end() {
            if self.match_token(Token::Newline) {
                continue;
            }

            let before = self.current;
            match self.parse_statement() {
                Ok(stmt) => stmts.push(stmt),
                Err(err) => {
                    self.record(err);
                    self.synchronize();
                }
            }
            // Progress guard: if nothing was consumed, force advance to
            // prevent an infinite loop on a completely unknown token.
            if self.current == before {
                self.advance();
            }
        }

        self.consume(Token::Dedent, "expected dedent after block")?;
        Ok(stmts)
    }

    // ── Control flow ──────────────────────────────────────────────────────────

    fn parse_if(&mut self) -> Result<Stmt, ParseError> {
        let if_span = self.previous_span();
        let cond = self.parse_expr()?;
        self.consume(Token::Colon, "expected ':' after if condition")?;
        self.consume_newlines();
        let then = self.parse_block()?;

        let mut elifs = Vec::new();
        while self.match_token(Token::Elif) {
            let c = self.parse_expr()?;
            self.consume(Token::Colon, "expected ':' after elif condition")?;
            self.consume_newlines();
            elifs.push((c, self.parse_block()?));
        }

        let else_ = if self.match_token(Token::Else) {
            self.consume(Token::Colon, "expected ':' after else")?;
            self.consume_newlines();
            Some(self.parse_block()?)
        } else {
            None
        };

        let end_span = else_
            .as_ref()
            .and_then(|block| block.last().map(|stmt| stmt.span))
            .or_else(|| {
                elifs
                    .last()
                    .and_then(|(_, block)| block.last().map(|stmt| stmt.span))
            })
            .or_else(|| then.last().map(|stmt| stmt.span))
            .unwrap_or(cond.span);

        Ok(Stmt::new(
            StmtKind::If {
                cond,
                then,
                elifs,
                else_,
            },
            if_span.join(end_span),
        ))
    }

    fn parse_while(&mut self) -> Result<Stmt, ParseError> {
        let while_span = self.previous_span();
        let cond = self.parse_expr()?;
        self.consume(Token::Colon, "expected ':' after while condition")?;
        self.consume_newlines();
        let body = self.parse_block()?;
        let end_span = body.last().map(|stmt| stmt.span).unwrap_or(cond.span);
        Ok(Stmt::new(
            StmtKind::While { cond, body },
            while_span.join(end_span),
        ))
    }

    fn parse_for(&mut self) -> Result<Stmt, ParseError> {
        let for_span = self.previous_span();
        let (var1, var1_span) = self.consume_ident("expected loop variable")?;

        if self.match_token(Token::Comma) {
            let (var2, var2_span) = self.consume_ident("expected second loop variable")?;
            self.consume(Token::In, "expected 'in' in for-kv loop")?;
            let iter = self.parse_expr()?;
            if self.check(&Token::Step) {
                return Err(ParseError::new(
                    "'step' is not supported in key-value for loops",
                    self.peek_span(),
                ));
            }
            self.consume(Token::Colon, "expected ':' after for-kv expression")?;
            self.consume_newlines();
            let body = self.parse_block()?;
            let end_span = body.last().map(|stmt| stmt.span).unwrap_or(iter.span);
            return Ok(Stmt::new(
                StmtKind::ForKV {
                    key: var1,
                    key_span: var1_span,
                    val: var2,
                    val_span: var2_span,
                    iter,
                    body,
                },
                for_span.join(end_span),
            ));
        }

        self.consume(Token::In, "expected 'in' in for loop")?;
        let iter = self.parse_expr()?;
        let step = if self.match_token(Token::Step) {
            Some(self.parse_expr()?)
        } else {
            None
        };
        self.consume(Token::Colon, "expected ':' after for-in expression")?;
        self.consume_newlines();
        let body = self.parse_block()?;
        let end_span = body.last().map(|stmt| stmt.span).unwrap_or(iter.span);
        Ok(Stmt::new(
            StmtKind::For {
                var: var1,
                var_span: var1_span,
                iter,
                step,
                body,
            },
            for_span.join(end_span),
        ))
    }

    fn parse_load(&mut self) -> Result<Stmt, ParseError> {
        let load_span = self.previous_span();
        let (alias, alias_span) = self.consume_ident("expected module name in load")?;
        let (source, source_span) = if self.match_token(Token::From) {
            let (source, span) = self.consume_str_literal("expected source string after 'from'")?;
            (source, Some(span))
        } else {
            (alias.clone(), None)
        };
        self.expect_stmt_end("after load")?;
        let end_span = source_span.unwrap_or(alias_span);
        Ok(Stmt::new(
            StmtKind::Load {
                alias,
                alias_span,
                source,
                source_span,
            },
            load_span.join(end_span),
        ))
    }

    // ── Types ─────────────────────────────────────────────────────────────────

    fn is_type_start(&self) -> bool {
        matches!(
            self.peek(),
            Token::Ident(_) | Token::LBracket | Token::LBrace | Token::Error
        )
    }

    /// Attempt to parse a type, rolling back silently if the tokens don't form
    /// a valid type. Never returns a hard error.
    fn try_parse_type(&mut self) -> Result<Option<Type>, ParseError> {
        if !self.is_type_start() {
            return Ok(None);
        }
        let checkpoint = self.current;
        match self.parse_type() {
            Ok(ty) => Ok(Some(ty)),
            Err(_) => {
                self.current = checkpoint;
                Ok(None)
            }
        }
    }

    fn parse_type(&mut self) -> Result<Type, ParseError> {
        let mut tys = vec![self.parse_type_primary()?];
        while self.match_token(Token::Pipe) {
            tys.push(self.parse_type_primary()?);
        }
        Ok(if tys.len() == 1 {
            tys.remove(0)
        } else {
            Type::Union(tys)
        })
    }

    fn parse_type_primary(&mut self) -> Result<Type, ParseError> {
        let span = self.peek_span();

        if self.match_token(Token::LBracket) {
            let inner = self.parse_type()?;
            self.consume(Token::RBracket, "expected ']' in list type")?;
            return Ok(Type::List(Box::new(inner)));
        }

        if self.match_token(Token::LBrace) {
            let key = self.parse_type()?;
            self.consume(Token::Colon, "expected ':' in map type")?;
            let val = self.parse_type()?;
            self.consume(Token::RBrace, "expected '}' in map type")?;
            return Ok(Type::Map(Box::new(key), Box::new(val)));
        }

        match self.advance() {
            Token::Error => Ok(Type::Error(Box::new(Type::Any))),
            Token::Ident(name) => Ok(match name.as_str() {
                "int" => Type::Int,
                "float" => Type::Float,
                "str" => Type::Str,
                "bool" => Type::Bool,
                "null" => Type::Null,
                "any" => Type::Any,
                _ => Type::Struct(name),
            }),
            Token::Null => Ok(Type::Null),
            other => Err(ParseError::unexpected_token(
                "a type annotation",
                Self::actual_name(&other),
                span,
            )),
        }
    }

    // ── String interpolation ──────────────────────────────────────────────────

    fn parse_interpolated_string(&mut self, raw: &str, span: Span) -> Result<Expr, ParseError> {
        if !raw.contains('{') {
            return Ok(Expr::new(ExprKind::Str(raw.to_string()), span));
        }

        let mut parts: Vec<InterpPart> = Vec::new();
        let mut buf = String::new();
        let mut chars = raw.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                '{' if matches!(chars.peek(), Some('{')) => {
                    chars.next();
                    buf.push('{');
                }
                '{' => {
                    if !buf.is_empty() {
                        parts.push(InterpPart::Str(std::mem::take(&mut buf)));
                    }
                    let src = Self::consume_interpolation(&mut chars)
                        .map_err(|_| ParseError::new("unterminated string interpolation", span))?;
                    let mut expr = Self::parse_inline_expr(&src)
                        .map_err(|err| ParseError::new(err.message, span))?;
                    Self::overwrite_expr_span(&mut expr, span);
                    parts.push(InterpPart::Expr(expr));
                }
                '}' if matches!(chars.peek(), Some('}')) => {
                    chars.next();
                    buf.push('}');
                }
                other => buf.push(other),
            }
        }

        if !buf.is_empty() {
            parts.push(InterpPart::Str(buf));
        }

        Ok(match parts.as_slice() {
            [InterpPart::Str(s)] => Expr::new(ExprKind::Str(s.clone()), span),
            _ => Expr::new(ExprKind::Interp(parts), span),
        })
    }

    fn consume_interpolation(
        chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    ) -> Result<String, ParseError> {
        let mut out = String::new();
        let mut depth: usize = 0;
        let mut in_str: Option<char> = None;

        while let Some(c) = chars.next() {
            if let Some(q) = in_str {
                out.push(c);
                if c == '\\' {
                    // Consume escape so a `\"` inside `{}` doesn't close the string.
                    if let Some(esc) = chars.next() {
                        out.push(esc);
                    }
                    continue;
                }
                if c == q {
                    in_str = None;
                }
                continue;
            }
            match c {
                '"' | '\'' => {
                    in_str = Some(c);
                    out.push(c);
                }
                '{' => {
                    depth += 1;
                    out.push(c);
                }
                '}' if depth > 0 => {
                    depth -= 1;
                    out.push(c);
                }
                '}' => return Ok(out), // depth == 0: end of interpolation
                _ => out.push(c),
            }
        }

        Err(ParseError::new(
            "unterminated string interpolation",
            Span::default(),
        ))
    }

    fn parse_inline_expr(src: &str) -> Result<Expr, ParseError> {
        let tokens = Lexer::new(src)
            .tokenize()
            .map_err(|e| ParseError::new(e.message, e.span))?;
        let mut p = Parser::new(tokens);
        let expr = p.parse_expr()?;
        if !p.is_at_end() && !matches!(p.peek(), Token::Newline) {
            return Err(ParseError::unexpected_token(
                "the end of this interpolation",
                Self::actual_name(p.peek()),
                p.peek_span(),
            ));
        }
        Ok(expr)
    }

    fn overwrite_expr_span(expr: &mut Expr, span: Span) {
        expr.span = span;
        match &mut expr.kind {
            ExprKind::BinOp { left, right, .. } => {
                Self::overwrite_expr_span(left, span);
                Self::overwrite_expr_span(right, span);
            }
            ExprKind::UnaryOp { expr, .. } => Self::overwrite_expr_span(expr, span),
            ExprKind::Call { callee, args } => {
                Self::overwrite_expr_span(callee, span);
                for arg in args {
                    Self::overwrite_expr_span(&mut arg.expr, span);
                }
            }
            ExprKind::Index { obj, idx } => {
                Self::overwrite_expr_span(obj, span);
                Self::overwrite_expr_span(idx, span);
            }
            ExprKind::Field {
                obj, field_span, ..
            } => {
                *field_span = span;
                Self::overwrite_expr_span(obj, span);
            }
            ExprKind::List(items) => {
                for item in items {
                    Self::overwrite_expr_span(item, span);
                }
            }
            ExprKind::Map(pairs) => {
                for (key, value) in pairs {
                    Self::overwrite_expr_span(key, span);
                    Self::overwrite_expr_span(value, span);
                }
            }
            ExprKind::Lambda { params, body, .. } => {
                for param in params {
                    param.span = span;
                    if param.ty.is_some() {
                        param.ty_span = Some(span);
                    }
                }
                for stmt in body {
                    Self::overwrite_stmt_span(stmt, span);
                }
            }
            ExprKind::Interp(parts) => {
                for part in parts {
                    if let InterpPart::Expr(expr) = part {
                        Self::overwrite_expr_span(expr, span);
                    }
                }
            }
            ExprKind::Ternary { cond, then, else_ } => {
                Self::overwrite_expr_span(cond, span);
                Self::overwrite_expr_span(then, span);
                Self::overwrite_expr_span(else_, span);
            }
            ExprKind::Propagate(inner) | ExprKind::Spawn(inner) => {
                Self::overwrite_expr_span(inner, span);
            }
            ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::Str(_)
            | ExprKind::Bool(_)
            | ExprKind::Null
            | ExprKind::Ident(_) => {}
        }
    }

    fn overwrite_stmt_span(stmt: &mut Stmt, span: Span) {
        stmt.span = span;
        match &mut stmt.kind {
            StmtKind::Assign {
                target_span,
                ty,
                ty_span,
                value,
                ..
            } => {
                *target_span = span;
                if ty.is_some() {
                    *ty_span = Some(span);
                }
                Self::overwrite_expr_span(value, span);
            }
            StmtKind::FieldAssign {
                obj,
                field_span,
                value,
                ..
            } => {
                *field_span = span;
                Self::overwrite_expr_span(obj, span);
                Self::overwrite_expr_span(value, span);
            }
            StmtKind::IndexAssign { obj, idx, value } => {
                Self::overwrite_expr_span(obj, span);
                Self::overwrite_expr_span(idx, span);
                Self::overwrite_expr_span(value, span);
            }
            StmtKind::Return(expr) => {
                if let Some(expr) = expr {
                    Self::overwrite_expr_span(expr, span);
                }
            }
            StmtKind::If {
                cond,
                then,
                elifs,
                else_,
            } => {
                Self::overwrite_expr_span(cond, span);
                for stmt in then {
                    Self::overwrite_stmt_span(stmt, span);
                }
                for (cond, block) in elifs {
                    Self::overwrite_expr_span(cond, span);
                    for stmt in block {
                        Self::overwrite_stmt_span(stmt, span);
                    }
                }
                if let Some(block) = else_ {
                    for stmt in block {
                        Self::overwrite_stmt_span(stmt, span);
                    }
                }
            }
            StmtKind::For {
                var_span,
                iter,
                step,
                body,
                ..
            } => {
                *var_span = span;
                Self::overwrite_expr_span(iter, span);
                if let Some(step) = step {
                    Self::overwrite_expr_span(step, span);
                }
                for stmt in body {
                    Self::overwrite_stmt_span(stmt, span);
                }
            }
            StmtKind::ForKV {
                key_span,
                val_span,
                iter,
                body,
                ..
            } => {
                *key_span = span;
                *val_span = span;
                Self::overwrite_expr_span(iter, span);
                for stmt in body {
                    Self::overwrite_stmt_span(stmt, span);
                }
            }
            StmtKind::While { cond, body } => {
                Self::overwrite_expr_span(cond, span);
                for stmt in body {
                    Self::overwrite_stmt_span(stmt, span);
                }
            }
            StmtKind::Expr(expr) => Self::overwrite_expr_span(expr, span),
            StmtKind::Load {
                alias_span,
                source_span,
                ..
            } => {
                *alias_span = span;
                if source_span.is_some() {
                    *source_span = Some(span);
                }
            }
            StmtKind::FnDef(fndef) => {
                fndef.name_span = span;
                if fndef.ret_ty.is_some() {
                    fndef.ret_ty_span = Some(span);
                }
                for param in &mut fndef.params {
                    param.span = span;
                    if param.ty.is_some() {
                        param.ty_span = Some(span);
                    }
                }
                for stmt in &mut fndef.body {
                    Self::overwrite_stmt_span(stmt, span);
                }
            }
            StmtKind::ClsDef(cls) => {
                cls.name_span = span;
                for field in &mut cls.fields {
                    field.span = span;
                    if field.ty.is_some() {
                        field.ty_span = Some(span);
                    }
                }
            }
            StmtKind::Export(inner) => Self::overwrite_stmt_span(inner, span),
            StmtKind::Break | StmtKind::Continue => {}
        }
    }
}

// ── Precedence ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Precedence {
    Lowest,
    Or,
    And,
    Equals,
    LessGreater,
    Range,
    Sum,
    Product,
    Prefix,
    Call,
    Field,
    Post,
}

impl Precedence {
    pub fn of(token: &Token) -> Self {
        match token {
            Token::Or => Self::Or,
            Token::And => Self::And,
            Token::EqEq | Token::BangEq => Self::Equals,
            Token::Lt | Token::Gt | Token::LtEq | Token::GtEq => Self::LessGreater,
            Token::DotDot => Self::Range,
            Token::Plus | Token::Minus => Self::Sum,
            Token::Star | Token::Slash | Token::Percent => Self::Product,
            Token::LParen => Self::Call,
            Token::Dot | Token::LBracket => Self::Field,
            Token::Question => Self::Post,
            _ => Self::Lowest,
        }
    }
}

// ── BinOp helper ─────────────────────────────────────────────────────────────

impl BinOp {
    fn from_token(token: &Token) -> Option<Self> {
        Some(match token {
            Token::Plus => Self::Plus,
            Token::Minus => Self::Minus,
            Token::Star => Self::Star,
            Token::Slash => Self::Slash,
            Token::Percent => Self::Percent,
            Token::EqEq => Self::EqEq,
            Token::BangEq => Self::BangEq,
            Token::Lt => Self::Lt,
            Token::Gt => Self::Gt,
            Token::LtEq => Self::LtEq,
            Token::GtEq => Self::GtEq,
            Token::And => Self::And,
            Token::Or => Self::Or,
            Token::DotDot => Self::DotDot,
            _ => return None,
        })
    }
}
