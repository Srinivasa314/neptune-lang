use crate::scanner::{Token, TokenType};
use num_enum::TryFromPrimitive;
use std::convert::TryInto;

pub struct Parser<'a, Tokens: Iterator<Item = Token<'a>>> {
    tokens: Tokens,
    current: Token<'a>,
    previous: Token<'a>,
    errors: Vec<CompileError>,
}

#[derive(Debug)]
pub struct CompileError {
    message: String,
    line: u32,
}

type ParseResult<T> = Result<T, CompileError>;

#[derive(TryFromPrimitive, Clone, Copy)]
#[repr(u8)]
enum Precedence {
    None,
    Assignment,
    Or,
    And,
    Equality,
    Comparison,
    Range,
    Additive,
    Multiplicative,
    Unary,
    Call,
    Primary,
}

fn get_precedence(token_type: &TokenType) -> Precedence {
    match token_type {
        TokenType::LeftParen => Precedence::Call,
        TokenType::RightParen => Precedence::None,
        TokenType::LeftSquareBracket => Precedence::Call,
        TokenType::RightSquareBracket => Precedence::None,
        TokenType::LeftBrace => Precedence::None,
        TokenType::RightBrace => Precedence::None,
        TokenType::Comma => Precedence::None,
        TokenType::Dot => Precedence::Call,
        TokenType::Minus => Precedence::Additive,
        TokenType::Plus => Precedence::Additive,
        TokenType::StatementSeparator => Precedence::None,
        TokenType::Slash => Precedence::Multiplicative,
        TokenType::Star => Precedence::Multiplicative,
        TokenType::Colon => Precedence::None,
        TokenType::HashBrace => Precedence::None,
        TokenType::DotDot => Precedence::Range,
        TokenType::Bang => Precedence::None,
        TokenType::BangEqual => Precedence::Comparison,
        TokenType::Equal => Precedence::Assignment,
        TokenType::EqualEqual => Precedence::Comparison,
        TokenType::Greater => Precedence::Comparison,
        TokenType::GreaterEqual => Precedence::Comparison,
        TokenType::Less => Precedence::Comparison,
        TokenType::LessEqual => Precedence::Comparison,
        TokenType::Identifier => Precedence::None,
        TokenType::String(_) => Precedence::None,
        TokenType::IntLiteral(_) => Precedence::None,
        TokenType::FloatLiteral(_) => Precedence::None,
        TokenType::Symbol(_) => Precedence::None,
        TokenType::And => Precedence::And,
        TokenType::Break => Precedence::None,
        TokenType::Class => Precedence::None,
        TokenType::Continue => Precedence::None,
        TokenType::Else => Precedence::None,
        TokenType::Extends => Precedence::None,
        TokenType::False => Precedence::None,
        TokenType::For => Precedence::None,
        TokenType::Fun => Precedence::None,
        TokenType::If => Precedence::None,
        TokenType::In => Precedence::Comparison,
        TokenType::Null => Precedence::None,
        TokenType::Or => Precedence::Or,
        TokenType::Return => Precedence::None,
        TokenType::Super => Precedence::None,
        TokenType::This => Precedence::None,
        TokenType::True => Precedence::None,
        TokenType::Let => Precedence::None,
        TokenType::Const => Precedence::None,
        TokenType::While => Precedence::None,
        TokenType::BeginString => Precedence::None,
        TokenType::EndString => Precedence::None,
        TokenType::Eof => Precedence::None,
        TokenType::Error(_) => Precedence::None,
    }
}

#[derive(Debug)]
pub enum Expr {
    Binary {
        left: Box<Expr>,
        op: TokenType,
        right: Box<Expr>,
        line: u32,
    },
    Unary {
        op: TokenType,
        right: Box<Expr>,
        line: u32,
    },
    Literal {
        inner: TokenType,
        line: u32,
    },
}

impl<'a, Tokens: Iterator<Item = Token<'a>>> Parser<'a, Tokens> {
    pub fn new(tokens: Tokens) -> Self {
        Self {
            tokens,
            current: Token::dummy_token(),
            previous: Token::dummy_token(),
            errors: vec![],
        }
    }

    pub fn exp_eof(&mut self) -> ParseResult<Expr> {
        let e = self.expression()?;
        self.consume(TokenType::Eof, "Expect end of expression".into())?;
        Ok(e)
    }

    pub fn parse(mut self) -> Result<Expr, Vec<CompileError>> {
        self.advance();
        match self.exp_eof() {
            Ok(exp) => {
                if self.errors.is_empty() {
                    Ok(exp)
                } else {
                    Err(self.errors)
                }
            }
            Err(e) => {
                self.errors.push(e);
                Err(self.errors)
            }
        }
    }

    fn advance(&mut self) {
        self.previous = self.current.clone();
        loop {
            match self.tokens.next() {
                Some(Token {
                    token_type: TokenType::Error(message),
                    line,
                    ..
                }) => self.errors.push(CompileError { message, line }),
                Some(t) => {
                    self.current = t;
                    break;
                }
                None => {
                    self.current = Token::dummy_token();
                    break;
                }
            }
        }
    }

    fn error(mut message: String, token: Token) -> CompileError {
        if token.token_type == TokenType::Eof {
            message.push_str(" at end");
        } else {
            message = format!("{} at token {}", message, token.inner)
        }
        CompileError {
            line: token.line,
            message,
        }
    }

    fn error_at_current(&self, message: String) -> CompileError {
        Self::error(message, self.current.clone())
    }

    fn error_at_previous(&self, message: String) -> CompileError {
        Self::error(message, self.previous.clone())
    }

    fn line(&self) -> u32 {
        self.previous.line
    }

    fn consume(&mut self, ttype: TokenType, message: String) -> ParseResult<()> {
        if self.current.token_type == ttype {
            self.advance();
            Ok(())
        } else {
            Err(self.error_at_current(message))
        }
    }

    fn parse_precedence(&mut self, prec: Precedence) -> ParseResult<Expr> {
        self.advance();
        if let Some(mut expr) = self.prefix(self.previous.token_type.clone()) {
            while prec as u8 <= get_precedence(&self.current.token_type) as u8 {
                self.advance();
                let op = self.previous.token_type.clone();
                expr = self.infix(op, Box::new(expr?));
            }
            expr
        } else {
            Err(self.error_at_previous("Expect expression".into()))
        }
    }

    fn prefix(&mut self, token_type: TokenType) -> Option<ParseResult<Expr>> {
        match token_type {
            TokenType::LeftParen => Some(self.grouping()),
            TokenType::RightParen => None,
            TokenType::LeftSquareBracket => Some(todo!()),
            TokenType::RightSquareBracket => None,
            TokenType::LeftBrace => None,
            TokenType::RightBrace => None,
            TokenType::Comma => None,
            TokenType::Dot => None,
            TokenType::Minus => Some(self.unary()),
            TokenType::Plus => None,
            TokenType::StatementSeparator => None,
            TokenType::Slash => None,
            TokenType::Star => None,
            TokenType::Colon => None,
            TokenType::HashBrace => Some(todo!()),
            TokenType::DotDot => None,
            TokenType::Bang => Some(todo!()),
            TokenType::BangEqual => None,
            TokenType::Equal => None,
            TokenType::EqualEqual => None,
            TokenType::Greater => None,
            TokenType::GreaterEqual => None,
            TokenType::Less => None,
            TokenType::LessEqual => None,
            TokenType::Identifier => Some(todo!()),
            TokenType::String(_) => None,
            TokenType::IntLiteral(_) => Some(self.literal()),
            TokenType::FloatLiteral(_) => Some(self.literal()),
            TokenType::Symbol(_) => Some(self.literal()),
            TokenType::And => None,
            TokenType::Break => None,
            TokenType::Class => None,
            TokenType::Continue => None,
            TokenType::Else => None,
            TokenType::Extends => None,
            TokenType::False => Some(self.literal()),
            TokenType::For => None,
            TokenType::Fun => None,
            TokenType::If => None,
            TokenType::In => None,
            TokenType::Null => Some(self.literal()),
            TokenType::Or => None,
            TokenType::Return => None,
            TokenType::Super => Some(todo!()),
            TokenType::This => Some(todo!()),
            TokenType::True => Some(self.literal()),
            TokenType::Let => None,
            TokenType::Const => None,
            TokenType::While => None,
            TokenType::BeginString => Some(todo!()),
            TokenType::EndString => None,
            TokenType::Eof => None,
            TokenType::Error(_) => None,
        }
    }

    fn infix(&mut self, token_type: TokenType, left: Box<Expr>) -> ParseResult<Expr> {
        match token_type {
            TokenType::LeftParen => todo!(),
            TokenType::RightParen => unreachable!(),
            TokenType::LeftSquareBracket => todo!(),
            TokenType::RightSquareBracket => unreachable!(),
            TokenType::LeftBrace => unreachable!(),
            TokenType::RightBrace => unreachable!(),
            TokenType::Comma => unreachable!(),
            TokenType::Dot => todo!(),
            TokenType::Minus => self.binary(left),
            TokenType::Plus => self.binary(left),
            TokenType::StatementSeparator => unreachable!(),
            TokenType::Slash => self.binary(left),
            TokenType::Star => self.binary(left),
            TokenType::Colon => unreachable!(),
            TokenType::HashBrace => unreachable!(),
            TokenType::DotDot => todo!(),
            TokenType::Bang => unreachable!(),
            TokenType::BangEqual => self.binary(left),
            TokenType::Equal => todo!(),
            TokenType::EqualEqual => self.binary(left),
            TokenType::Greater => self.binary(left),
            TokenType::GreaterEqual => self.binary(left),
            TokenType::Less => self.binary(left),
            TokenType::LessEqual => self.binary(left),
            TokenType::Identifier => unreachable!(),
            TokenType::String(_) => unreachable!(),
            TokenType::IntLiteral(_) => unreachable!(),
            TokenType::FloatLiteral(_) => unreachable!(),
            TokenType::Symbol(_) => unreachable!(),
            TokenType::And => self.binary(left),
            TokenType::Break => unreachable!(),
            TokenType::Class => unreachable!(),
            TokenType::Continue => unreachable!(),
            TokenType::Else => unreachable!(),
            TokenType::Extends => unreachable!(),
            TokenType::False => unreachable!(),
            TokenType::For => unreachable!(),
            TokenType::Fun => unreachable!(),
            TokenType::If => unreachable!(),
            TokenType::In => self.binary(left),
            TokenType::Null => unreachable!(),
            TokenType::Or => self.binary(left),
            TokenType::Return => unreachable!(),
            TokenType::Super => unreachable!(),
            TokenType::This => unreachable!(),
            TokenType::True => unreachable!(),
            TokenType::Let => unreachable!(),
            TokenType::Const => unreachable!(),
            TokenType::While => unreachable!(),
            TokenType::BeginString => unreachable!(),
            TokenType::EndString => unreachable!(),
            TokenType::Eof => unreachable!(),
            TokenType::Error(_) => unreachable!(),
        }
    }

    fn expression(&mut self) -> ParseResult<Expr> {
        self.parse_precedence(Precedence::Assignment)
    }

    fn grouping(&mut self) -> ParseResult<Expr> {
        let expr = self.expression()?;
        self.consume(TokenType::RightParen, "Expect ')' after expression".into())?;
        Ok(expr)
    }

    fn unary(&mut self) -> ParseResult<Expr> {
        let op = self.previous.token_type.clone();
        Ok(Expr::Unary {
            op,
            right: Box::new(self.parse_precedence(Precedence::Unary)?),
            line: self.line(),
        })
    }

    fn binary(&mut self, left: Box<Expr>) -> ParseResult<Expr> {
        let op = self.previous.token_type.clone();
        let right =
            Box::new(self.parse_precedence((get_precedence(&op) as u8 + 1).try_into().unwrap())?);
        Ok(Expr::Binary {
            left,
            op,
            right,
            line: self.line(),
        })
    }

    fn literal(&self) -> ParseResult<Expr> {
        Ok(Expr::Literal {
            inner: self.previous.token_type.clone(),
            line: self.line(),
        })
    }
}
