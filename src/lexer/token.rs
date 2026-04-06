#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Literals
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Null,

    // Identifiers & Keywords
    Ident(String),
    Fn,
    Cls,
    Load,
    From,
    As,
    Export,
    Return,
    If,
    Elif,
    Else,
    For,
    While,
    In,
    Break,
    Continue,
    Spawn,
    Error,
    Step,

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Assign,
    EqEq,
    BangEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
    Not,
    Arrow,
    Question,
    DotDot,
    Dot,
    Pipe,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,

    // Structure
    Indent,
    Dedent,
    Newline,
    Colon,
    Comma,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,

    // Interp
    InterpStart,
    InterpEnd,

    Eof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start: usize,
    pub len: usize,
}

impl Span {
    pub const fn new(start: usize, len: usize) -> Self {
        Self { start, len }
    }

    pub const fn end(self) -> usize {
        self.start + self.len
    }

    pub fn join(self, other: Self) -> Self {
        let start = self.start.min(other.start);
        let end = self.end().max(other.end());
        Self::new(start, end.saturating_sub(start))
    }

    pub const fn point(offset: usize) -> Self {
        Self::new(offset, 0)
    }
}

impl Default for Token {
    fn default() -> Self {
        Token::Eof
    }
}
