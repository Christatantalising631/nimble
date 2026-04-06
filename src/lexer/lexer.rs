use super::token::{Span, Token};
use crate::error::LexError;
use std::iter::Peekable;
use std::str::CharIndices;

pub struct Lexer<'a> {
    source_text: &'a str,
    chars: Peekable<CharIndices<'a>>,
    offset: usize,
    indent_stack: Vec<usize>,
    pending_tokens: Vec<(Token, Span)>,
    at_line_start: bool,
    paren_level: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source_text: source,
            chars: source.char_indices().peekable(),
            offset: 0,
            indent_stack: vec![0],
            pending_tokens: Vec::new(),
            at_line_start: true,
            paren_level: 0,
        }
    }

    fn peek(&mut self) -> Option<char> {
        self.chars.peek().map(|(_, ch)| *ch)
    }

    fn current_char_span(&mut self) -> Span {
        match self.chars.peek() {
            Some((idx, ch)) => Span::new(*idx, ch.len_utf8()),
            None => Span::point(self.source_text.len()),
        }
    }

    fn advance(&mut self) -> Option<(usize, char)> {
        let (idx, ch) = self.chars.next()?;
        self.offset = idx + ch.len_utf8();
        Some((idx, ch))
    }

    pub fn tokenize(&mut self) -> Result<Vec<(Token, Span)>, LexError> {
        let mut tokens = Vec::new();
        loop {
            let (token, span) = self.next_token()?;
            let is_eof = token == Token::Eof;
            tokens.push((token, span));
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }

    fn next_token(&mut self) -> Result<(Token, Span), LexError> {
        if !self.pending_tokens.is_empty() {
            return Ok(self.pending_tokens.remove(0));
        }

        if self.at_line_start {
            self.handle_indentation()?;
            if !self.pending_tokens.is_empty() {
                return Ok(self.pending_tokens.remove(0));
            }
        }

        self.skip_whitespace();
        let (token, span) = match self.advance() {
            Some((start, c)) => match c {
                '#' => {
                    while let Some(pc) = self.peek() {
                        if pc == '\n' {
                            break;
                        }
                        self.advance();
                    }
                    return self.next_token();
                }
                '\n' => {
                    self.at_line_start = true;
                    if self.paren_level == 0 {
                        (Token::Newline, Span::new(start, 1))
                    } else {
                        return self.next_token();
                    }
                }
                '(' => {
                    self.paren_level += 1;
                    (Token::LParen, Span::new(start, 1))
                }
                ')' => {
                    if self.paren_level > 0 {
                        self.paren_level -= 1;
                    }
                    (Token::RParen, Span::new(start, 1))
                }
                '{' => {
                    self.paren_level += 1;
                    (Token::LBrace, Span::new(start, 1))
                }
                '}' => {
                    if self.paren_level > 0 {
                        self.paren_level -= 1;
                    }
                    (Token::RBrace, Span::new(start, 1))
                }
                '[' => {
                    self.paren_level += 1;
                    (Token::LBracket, Span::new(start, 1))
                }
                ']' => {
                    if self.paren_level > 0 {
                        self.paren_level -= 1;
                    }
                    (Token::RBracket, Span::new(start, 1))
                }
                ',' => (Token::Comma, Span::new(start, 1)),
                ':' => (Token::Colon, Span::new(start, 1)),
                '?' => (Token::Question, Span::new(start, 1)),
                '|' => (Token::Pipe, Span::new(start, 1)),
                '.' => {
                    if self.peek() == Some('.') {
                        self.advance();
                        (Token::DotDot, Span::new(start, self.offset - start))
                    } else {
                        (Token::Dot, Span::new(start, 1))
                    }
                }
                '+' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        (Token::PlusEq, Span::new(start, self.offset - start))
                    } else {
                        (Token::Plus, Span::new(start, 1))
                    }
                }
                '-' => match self.peek() {
                    Some('=') => {
                        self.advance();
                        (Token::MinusEq, Span::new(start, self.offset - start))
                    }
                    Some('>') => {
                        self.advance();
                        (Token::Arrow, Span::new(start, self.offset - start))
                    }
                    _ => (Token::Minus, Span::new(start, 1)),
                },
                '*' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        (Token::StarEq, Span::new(start, self.offset - start))
                    } else {
                        (Token::Star, Span::new(start, 1))
                    }
                }
                '/' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        (Token::SlashEq, Span::new(start, self.offset - start))
                    } else {
                        (Token::Slash, Span::new(start, 1))
                    }
                }
                '%' => (Token::Percent, Span::new(start, 1)),
                '=' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        (Token::EqEq, Span::new(start, self.offset - start))
                    } else {
                        (Token::Assign, Span::new(start, 1))
                    }
                }
                '!' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        (Token::BangEq, Span::new(start, self.offset - start))
                    } else {
                        return Err(LexError::invalid_token("!", Span::new(start, 1)));
                    }
                }
                '<' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        (Token::LtEq, Span::new(start, self.offset - start))
                    } else {
                        (Token::Lt, Span::new(start, 1))
                    }
                }
                '>' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        (Token::GtEq, Span::new(start, self.offset - start))
                    } else {
                        (Token::Gt, Span::new(start, 1))
                    }
                }
                '"' | '\'' => return self.lex_string(c, start),
                c if c.is_ascii_digit() => {
                    let token = self.lex_number(c, start)?;
                    (token, Span::new(start, self.offset - start))
                }
                c if c.is_alphabetic() || c == '_' => {
                    let token = self.lex_identifier(c);
                    (token, Span::new(start, self.offset - start))
                }
                other => {
                    return Err(LexError::invalid_token(
                        other.to_string(),
                        Span::new(start, other.len_utf8()),
                    ));
                }
            },
            None => {
                while self.indent_stack.len() > 1 {
                    self.indent_stack.pop();
                    self.pending_tokens
                        .push((Token::Dedent, Span::point(self.offset)));
                }
                return if !self.pending_tokens.is_empty() {
                    Ok(self.pending_tokens.remove(0))
                } else {
                    Ok((Token::Eof, Span::point(self.offset)))
                };
            }
        };

        Ok((token, span))
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if matches!(c, ' ' | '\t' | '\r') {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn handle_indentation(&mut self) -> Result<(), LexError> {
        self.at_line_start = false;
        let mut indent = 0usize;

        while let Some(c) = self.peek() {
            if c == ' ' {
                indent += 1;
                self.advance();
            } else if c == '\t' {
                indent += 4;
                self.advance();
            } else {
                break;
            }
        }

        if matches!(self.peek(), Some('\n' | '\r' | '#')) || self.peek().is_none() {
            return Ok(());
        }

        let span = self.current_char_span();
        let last_indent = self.indent_stack.last().copied().unwrap_or(0);

        if indent > last_indent {
            self.indent_stack.push(indent);
            self.pending_tokens.push((Token::Indent, span));
        } else if indent < last_indent {
            while indent < self.indent_stack.last().copied().unwrap_or(0) {
                self.indent_stack.pop();
                self.pending_tokens.push((Token::Dedent, span));
            }

            let expected = self.indent_stack.last().copied().unwrap_or(0);
            if indent != expected {
                return Err(LexError::indentation(expected, indent, span));
            }
        }

        Ok(())
    }

    fn lex_string(&mut self, quote: char, start: usize) -> Result<(Token, Span), LexError> {
        let mut s = String::new();

        while let Some((idx, c)) = self.advance() {
            if c == quote {
                return Ok((Token::Str(s), Span::new(start, self.offset - start)));
            }

            if c == '\\' {
                match self.advance() {
                    Some((_, 'n')) => s.push('\n'),
                    Some((_, 'r')) => s.push('\r'),
                    Some((_, 't')) => s.push('\t'),
                    Some((_, '\\')) => s.push('\\'),
                    Some((_, '"')) => s.push('"'),
                    Some((_, '\'')) => s.push('\''),
                    Some((_, other)) => s.push(other),
                    None => {
                        return Err(LexError::unterminated_string_escape(Span::new(
                            idx,
                            self.source_text.len().saturating_sub(idx),
                        )));
                    }
                }
            } else {
                s.push(c);
            }
        }

        Err(LexError::unterminated_string(Span::new(
            start,
            self.source_text.len().saturating_sub(start),
        )))
    }

    fn lex_number(&mut self, first: char, start: usize) -> Result<Token, LexError> {
        let mut s = first.to_string();
        let mut is_float = false;

        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                if let Some((_, ch)) = self.advance() {
                    s.push(ch);
                }
            } else if c == '.' && !is_float {
                let mut probe = self.chars.clone();
                probe.next();
                if matches!(probe.next(), Some((_, '.'))) {
                    break;
                }
                is_float = true;
                if let Some((_, ch)) = self.advance() {
                    s.push(ch);
                }
            } else {
                break;
            }
        }

        let span = Span::new(start, self.offset.saturating_sub(start));
        if is_float {
            s.parse::<f64>()
                .map(Token::Float)
                .map_err(|_| LexError::invalid_number(s.clone(), span))
        } else {
            s.parse::<i64>()
                .map(Token::Int)
                .map_err(|_| LexError::invalid_number(s, span))
        }
    }

    fn lex_identifier(&mut self, first: char) -> Token {
        let mut s = first.to_string();

        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                if let Some((_, ch)) = self.advance() {
                    s.push(ch);
                }
            } else {
                break;
            }
        }

        match s.as_str() {
            "fn" => Token::Fn,
            "cls" => Token::Cls,
            "load" => Token::Load,
            "from" => Token::From,
            "as" => Token::As,
            "export" => Token::Export,
            "return" => Token::Return,
            "if" => Token::If,
            "elif" => Token::Elif,
            "else" => Token::Else,
            "for" => Token::For,
            "while" => Token::While,
            "in" => Token::In,
            "break" => Token::Break,
            "continue" => Token::Continue,
            "spawn" => Token::Spawn,
            "error" => Token::Error,
            "step" => Token::Step,
            "true" => Token::Bool(true),
            "false" => Token::Bool(false),
            "null" => Token::Null,
            "and" => Token::And,
            "or" => Token::Or,
            "not" => Token::Not,
            _ => Token::Ident(s),
        }
    }
}
