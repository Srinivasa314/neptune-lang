use crate::{
    scanner::{Token, TokenType},
    CompileError, CompileResult,
};
use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;

pub struct Parser<'src, Tokens: Iterator<Item = Token<'src>>> {
    tokens: Tokens,
    current: Token<'src>,
    previous: Token<'src>,
    errors: Vec<CompileError>,
}

#[derive(TryFromPrimitive, Clone, Copy)]
#[repr(u8)]
enum Precedence {
    None,
    Assignment,
    Or,
    And,
    Equality,
    Comparison,
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
        TokenType::Dot => Precedence::None,
        TokenType::Minus => Precedence::Additive,
        TokenType::Plus => Precedence::Additive,
        TokenType::StatementSeparator => Precedence::None,
        TokenType::Slash => Precedence::Multiplicative,
        TokenType::Star => Precedence::Multiplicative,
        TokenType::Mod => Precedence::Multiplicative,
        TokenType::Colon => Precedence::None,
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
        TokenType::Const => Precedence::None,
        TokenType::Else => Precedence::None,
        TokenType::Extends => Precedence::None,
        TokenType::False => Precedence::None,
        TokenType::For => Precedence::None,
        TokenType::Fun => Precedence::None,
        TokenType::If => Precedence::None,
        TokenType::In => Precedence::None,
        TokenType::Null => Precedence::None,
        TokenType::Or => Precedence::Or,
        TokenType::Return => Precedence::None,
        TokenType::Super => Precedence::None,
        TokenType::This => Precedence::None,
        TokenType::True => Precedence::None,
        TokenType::Let => Precedence::None,
        TokenType::While => Precedence::None,
        TokenType::Interpolation => Precedence::None,
        TokenType::Eof => Precedence::None,
        TokenType::Error(_) => Precedence::None,
        TokenType::PlusEqual => Precedence::Assignment,
        TokenType::MinusEqual => Precedence::Assignment,
        TokenType::StarEqual => Precedence::Assignment,
        TokenType::SlashEqual => Precedence::Assignment,
        TokenType::ModEqual => Precedence::Assignment,
        TokenType::Tilde => Precedence::Additive,
        TokenType::TildeEqual => Precedence::Assignment,
        TokenType::EqualEqualEqual => Precedence::Comparison,
        TokenType::BangEqualEqual => Precedence::Comparison,
        TokenType::DotDot => Precedence::None,
        TokenType::Print => Precedence::None,
        TokenType::Pipe => Precedence::None,
    }
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Substring {
    String(String),
    Expr(Expr),
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Expr {
    Binary {
        line: u32,
        left: Box<Expr>,
        op: TokenType,
        right: Box<Expr>,
    },
    Unary {
        line: u32,
        op: TokenType,
        right: Box<Expr>,
    },
    Literal {
        line: u32,
        inner: TokenType,
    },
    Variable {
        line: u32,
        name: String,
    },
    String {
        line: u32,
        inner: Vec<Substring>,
    },
    Array {
        line: u32,
        inner: Vec<Expr>,
    },
    Subscript {
        line: u32,
        object: Box<Expr>,
        subscript: Box<Expr>,
    },
    Map {
        line: u32,
        inner: Vec<(Expr, Expr)>,
    },
    Call {
        line: u32,
        function: Box<Expr>,
        arguments: Vec<Expr>,
    },
    Closure {
        line: u32,
        last_line: u32,
        args: Vec<String>,
        body: ClosureBody,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClosureBody {
    Block(Vec<Statement>),
    Expr(Box<Expr>),
}

impl Expr {
    pub fn line(&self) -> u32 {
        match self {
            Expr::Binary { line, .. } => *line,
            Expr::Unary { line, .. } => *line,
            Expr::Literal { line, .. } => *line,
            Expr::Variable { line, .. } => *line,
            Expr::String { line, .. } => *line,
            Expr::Array { line, .. } => *line,
            Expr::Subscript { line, .. } => *line,
            Expr::Map { line, .. } => *line,
            Expr::Call { line, .. } => *line,
            Expr::Closure { line, .. } => *line,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Statement {
    Expr(Expr),
    VarDeclaration {
        name: String,
        expr: Expr,
        mutable: bool,
        line: u32,
    },
    Block {
        block: Vec<Statement>,
        end_line: u32,
    },
    If {
        condition: Expr,
        block: Vec<Statement>,
        else_stmt: Option<Box<Statement>>,
        if_end: u32,
    },
    While {
        condition: Expr,
        block: Vec<Statement>,
        end_line: u32,
    },
    For {
        begin_line: u32,
        iter: String,
        start: Box<Expr>,
        end: Box<Expr>,
        block: Vec<Statement>,
        end_line: u32,
    },
    Break {
        line: u32,
    },
    Continue {
        line: u32,
    },
    Function {
        line: u32,
        last_line: u32,
        name: String,
        arguments: Vec<String>,
        body: Vec<Statement>,
    },
    Return {
        line: u32,
        expr: Option<Expr>,
    },
    Print(Expr),
}

impl<'src, Tokens: Iterator<Item = Token<'src>>> Parser<'src, Tokens> {
    pub fn new(tokens: Tokens) -> Self {
        Self {
            tokens,
            current: Token::uninit_token(),
            previous: Token::uninit_token(),
            errors: vec![],
        }
    }

    pub fn parse(mut self, try_expr: bool) -> (Vec<Statement>, Vec<CompileError>) {
        self.advance();
        let mut statements = vec![];
        while self.current.token_type != TokenType::Eof {
            if let Some(stmt) = self.statement(true, try_expr) {
                statements.push(stmt);
            }
        }
        (statements, self.errors)
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
                    self.current = Token::uninit_token();
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

    fn consume(&mut self, ttype: TokenType, message: String) -> CompileResult<()> {
        if self.current.token_type == ttype {
            self.advance();
            Ok(())
        } else {
            Err(self.error_at_current(message))
        }
    }

    fn match_token(&mut self, ttype: TokenType) -> bool {
        if self.current.token_type != ttype {
            false
        } else {
            self.advance();
            true
        }
    }

    fn ignore_newline(&mut self) {
        if self.current.token_type == TokenType::StatementSeparator && self.current.inner == "\n" {
            self.advance();
        }
    }

    fn parse_precedence(&mut self, prec: Precedence) -> CompileResult<Expr> {
        self.advance();
        if let Some(expr) = self.prefix(self.previous.token_type.clone()) {
            let mut expr = expr?;
            while prec as u8 <= get_precedence(&self.current.token_type) as u8 {
                self.advance();
                let op = self.previous.token_type.clone();
                expr = self.infix(op, Box::new(expr))?;
            }
            Ok(expr)
        } else {
            Err(self.error_at_previous("Expect expression".into()))
        }
    }

    fn prefix(&mut self, token_type: TokenType) -> Option<CompileResult<Expr>> {
        match token_type {
            TokenType::LeftParen => Some(self.grouping()),
            TokenType::RightParen => None,
            TokenType::LeftSquareBracket => Some(self.array()),
            TokenType::RightSquareBracket => None,
            TokenType::LeftBrace => Some(self.map()),
            TokenType::RightBrace => None,
            TokenType::Comma => None,
            TokenType::Dot => None,
            TokenType::Minus => Some(self.unary()),
            TokenType::Plus => None,
            TokenType::StatementSeparator => None,
            TokenType::Slash => None,
            TokenType::Mod => None,
            TokenType::Star => None,
            TokenType::Colon => None,
            TokenType::Bang => Some(self.unary()),
            TokenType::BangEqual => None,
            TokenType::Equal => None,
            TokenType::EqualEqual => None,
            TokenType::Greater => None,
            TokenType::GreaterEqual => None,
            TokenType::Less => None,
            TokenType::LessEqual => None,
            TokenType::Identifier => Some(self.variable()),
            TokenType::String(_) => Some(self.string()),
            TokenType::IntLiteral(_) => Some(self.literal()),
            TokenType::FloatLiteral(_) => Some(self.literal()),
            TokenType::Symbol(_) => Some(self.literal()),
            TokenType::And => None,
            TokenType::Break => None,
            TokenType::Class => None,
            TokenType::Continue => None,
            TokenType::Const => None,
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
            TokenType::Super => todo!(),
            TokenType::This => todo!(),
            TokenType::True => Some(self.literal()),
            TokenType::Let => None,
            TokenType::While => None,
            TokenType::Interpolation => None,
            TokenType::Eof => None,
            TokenType::Error(_) => None,
            TokenType::PlusEqual => None,
            TokenType::MinusEqual => None,
            TokenType::StarEqual => None,
            TokenType::SlashEqual => None,
            TokenType::ModEqual => None,
            TokenType::Tilde => None,
            TokenType::TildeEqual => None,
            TokenType::EqualEqualEqual => None,
            TokenType::BangEqualEqual => None,
            TokenType::DotDot => None,
            TokenType::Print => None,
            TokenType::Pipe => Some(self.closure()),
        }
    }

    fn infix(&mut self, token_type: TokenType, left: Box<Expr>) -> CompileResult<Expr> {
        match token_type {
            TokenType::LeftParen => self.call(left),
            TokenType::RightParen => unreachable!(),
            TokenType::LeftSquareBracket => self.subscript(left),
            TokenType::RightSquareBracket => unreachable!(),
            TokenType::LeftBrace => unreachable!(),
            TokenType::RightBrace => unreachable!(),
            TokenType::Comma => unreachable!(),
            TokenType::Dot => unreachable!(),
            TokenType::Minus => self.binary(left),
            TokenType::Plus => self.binary(left),
            TokenType::StatementSeparator => unreachable!(),
            TokenType::Slash => self.binary(left),
            TokenType::Star => self.binary(left),
            TokenType::Mod => self.binary(left),
            TokenType::Colon => unreachable!(),
            TokenType::Bang => unreachable!(),
            TokenType::BangEqual => self.binary(left),
            TokenType::Equal => self.binary(left),
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
            TokenType::Const => unreachable!(),
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
            TokenType::While => unreachable!(),
            TokenType::Interpolation => unreachable!(),
            TokenType::Eof => unreachable!(),
            TokenType::Error(_) => unreachable!(),
            TokenType::PlusEqual => self.binary(left),
            TokenType::MinusEqual => self.binary(left),
            TokenType::StarEqual => self.binary(left),
            TokenType::SlashEqual => self.binary(left),
            TokenType::ModEqual => self.binary(left),
            TokenType::Tilde => self.binary(left),
            TokenType::TildeEqual => self.binary(left),
            TokenType::EqualEqualEqual => self.binary(left),
            TokenType::BangEqualEqual => self.binary(left),
            TokenType::DotDot => unreachable!(),
            TokenType::Print => unreachable!(),
            TokenType::Pipe => unreachable!(),
        }
    }

    fn expression(&mut self) -> CompileResult<Expr> {
        self.parse_precedence(Precedence::Assignment)
    }

    fn grouping(&mut self) -> CompileResult<Expr> {
        let expr = self.expression()?;
        self.consume(TokenType::RightParen, "Expect ')' after expression".into())?;
        Ok(expr)
    }

    fn unary(&mut self) -> CompileResult<Expr> {
        let op = self.previous.token_type.clone();
        Ok(Expr::Unary {
            op,
            right: Box::new(self.parse_precedence(Precedence::Unary)?),
            line: self.line(),
        })
    }

    fn array(&mut self) -> CompileResult<Expr> {
        let mut ret: Vec<Expr> = vec![];
        let line = self.previous.line;
        loop {
            if self.current.token_type == TokenType::RightSquareBracket {
                self.advance();
                break;
            }
            ret.push(self.expression()?);
            if self.match_token(TokenType::RightSquareBracket) {
                break;
            }
            self.consume(TokenType::Comma, "Expect comma after array element".into())?;
        }
        Ok(Expr::Array { inner: ret, line })
    }

    fn call(&mut self, function: Box<Expr>) -> CompileResult<Expr> {
        let mut arguments: Vec<Expr> = vec![];
        let line = self.previous.line;
        loop {
            if self.current.token_type == TokenType::RightParen {
                self.advance();
                break;
            }
            arguments.push(self.expression()?);
            if self.match_token(TokenType::RightParen) {
                break;
            }
            self.consume(TokenType::Comma, "Expect comma after argument".into())?;
        }
        Ok(Expr::Call {
            line,
            function,
            arguments,
        })
    }

    fn subscript(&mut self, left: Box<Expr>) -> CompileResult<Expr> {
        let subscript = self.expression()?;
        self.consume(
            TokenType::RightSquareBracket,
            "Expect ']' after expression".into(),
        )?;
        Ok(Expr::Subscript {
            object: left,
            subscript: Box::new(subscript),
            line: self.line(),
        })
    }

    fn map(&mut self) -> CompileResult<Expr> {
        let mut ret = vec![];
        let line = self.previous.line;
        loop {
            if self.current.token_type == TokenType::RightBrace {
                self.advance();
                break;
            }
            self.ignore_newline();
            let e1 = self.expression()?;
            self.ignore_newline();
            self.consume(TokenType::Colon, "Expect colon after map key".into())?;
            self.ignore_newline();
            let e2 = self.expression()?;
            self.ignore_newline();
            ret.push((e1, e2));
            if self.match_token(TokenType::RightBrace) {
                break;
            }
            self.consume(TokenType::Comma, "Expect comma after map value".into())?;
            self.ignore_newline();
        }
        Ok(Expr::Map { inner: ret, line })
    }

    fn function(&mut self) -> CompileResult<Statement> {
        self.consume(
            TokenType::Identifier,
            "Expect identifier for function name".into(),
        )?;
        let name = self.previous.inner.to_string();
        let line = self.previous.line;
        let mut arguments: Vec<String> = vec![];
        self.consume(
            TokenType::LeftParen,
            "Expect ( to begin argument list".into(),
        )?;
        loop {
            if self.current.token_type == TokenType::RightParen {
                self.advance();
                break;
            }
            self.consume(TokenType::Identifier, "Expect argument name".into())?;
            arguments.push(self.previous.inner.to_string());
            if self.match_token(TokenType::RightParen) {
                break;
            }
            self.consume(TokenType::Comma, "Expect comma after argument".into())?;
        }
        self.consume(
            TokenType::LeftBrace,
            "Expect { to begin function body".into(),
        )?;
        let body = self.block()?;
        let last_line = self.previous.line;
        Ok(Statement::Function {
            line,
            last_line,
            name,
            arguments,
            body,
        })
    }

    fn return_stmt(&mut self) -> CompileResult<Statement> {
        if self.current.token_type == TokenType::StatementSeparator {
            Ok(Statement::Return {
                line: self.previous.line,
                expr: None,
            })
        } else {
            Ok(Statement::Return {
                line: self.previous.line,
                expr: Some(self.expression()?),
            })
        }
    }

    fn binary(&mut self, left: Box<Expr>) -> CompileResult<Expr> {
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

    fn literal(&self) -> CompileResult<Expr> {
        Ok(Expr::Literal {
            inner: self.previous.token_type.clone(),
            line: self.line(),
        })
    }

    fn synchronize(&mut self) {
        while self.current.token_type != TokenType::Eof {
            if self.previous.token_type == TokenType::StatementSeparator {
                return;
            };
            match self.current.token_type {
                TokenType::Class
                | TokenType::Fun
                | TokenType::Let
                | TokenType::For
                | TokenType::If
                | TokenType::While
                | TokenType::Return => return,
                _ => {}
            }

            self.advance();
        }
    }
    fn expression_statement(&mut self) -> CompileResult<Statement> {
        let e = self.expression()?;
        Ok(Statement::Expr(e))
    }

    fn statement(&mut self, needs_sep: bool, try_expr: bool) -> Option<Statement> {
        match (|| {
            let e = if self.match_token(TokenType::Let) {
                self.var_declaration(true)
            } else if self.match_token(TokenType::Const) {
                self.var_declaration(false)
            } else if self.match_token(TokenType::LeftBrace) {
                if try_expr {
                    self.map().map(Statement::Expr)
                } else {
                    self.block().map(|block| Statement::Block {
                        block,
                        end_line: self.previous.line,
                    })
                }
            } else if self.match_token(TokenType::If) {
                self.if_statement()
            } else if self.match_token(TokenType::While) {
                self.while_loop()
            } else if self.match_token(TokenType::For) {
                self.for_loop()
            } else if self.match_token(TokenType::Break) {
                Ok(Statement::Break {
                    line: self.previous.line,
                })
            } else if self.match_token(TokenType::Continue) {
                Ok(Statement::Continue {
                    line: self.previous.line,
                })
            } else if self.match_token(TokenType::Fun) {
                self.function()
            } else if self.match_token(TokenType::Return) {
                self.return_stmt()
            } else if self.match_token(TokenType::Print) {
                Ok(Statement::Print(self.expression()?))
            } else {
                self.expression_statement()
            }?;
            if needs_sep
                && !(self.match_token(TokenType::StatementSeparator)
                    || self.match_token(TokenType::Eof))
            {
                return Err(self.error_at_current("Expect newline or semicolon".into()));
            }
            Ok(e)
        })() {
            Ok(s) => Some(s),
            Err(e) => {
                self.errors.push(e);
                self.synchronize();
                None
            }
        }
    }

    fn var_declaration(&mut self, mutable: bool) -> CompileResult<Statement> {
        if TokenType::Identifier == self.current.token_type {
            self.advance();
            let name = self.previous.inner.into();
            let line = self.previous.line;
            self.consume(TokenType::Equal, "Variable must be initialized".into())?;
            let expr = self.expression()?;
            Ok(Statement::VarDeclaration {
                name,
                expr,
                line,
                mutable,
            })
        } else {
            Err(self.error_at_current("Expect identifier".into()))
        }
    }

    fn string(&mut self) -> CompileResult<Expr> {
        let mut substrings = vec![];
        let line: u32;
        if let TokenType::String(s) = &self.previous.token_type {
            if !s.is_empty() {
                substrings.push(Substring::String(s.clone()));
            }
            line = self.previous.line;
            while self.match_token(TokenType::Interpolation) {
                substrings.push(Substring::Expr(self.expression()?));
                if let TokenType::String(s) = &self.current.token_type {
                    if !s.is_empty() {
                        substrings.push(Substring::String(s.clone()));
                    }
                    self.advance();
                } else {
                    unreachable!()
                }
            }
        } else {
            unreachable!()
        }
        Ok(Expr::String {
            inner: substrings,
            line,
        })
    }

    fn variable(&mut self) -> CompileResult<Expr> {
        Ok(Expr::Variable {
            name: self.previous.inner.into(),
            line: self.previous.line,
        })
    }

    fn block(&mut self) -> CompileResult<Vec<Statement>> {
        let mut statements = vec![];
        loop {
            if self.match_token(TokenType::Eof) {
                return Err(CompileError {
                    message: "Expect } after block".to_string(),
                    line: self.current.line,
                });
            } else if self.match_token(TokenType::RightBrace) {
                break;
            } else if let Some(stmt) = self.statement(false, false) {
                statements.push(stmt);
                if !matches!(
                    self.current.token_type,
                    TokenType::Eof | TokenType::RightBrace
                ) {
                    self.consume(
                        TokenType::StatementSeparator,
                        "Expect newline or semicolon after statement".into(),
                    )?;
                }
            }
        }
        Ok(statements)
    }

    fn if_statement(&mut self) -> CompileResult<Statement> {
        let condition = self.expression()?;
        self.ignore_newline();
        self.consume(
            TokenType::LeftBrace,
            "Expect { after condition in if statement".into(),
        )?;
        let block = self.block()?;
        let if_end = self.previous.line;
        let else_stmt = if self.match_token(TokenType::Else) {
            let s = self.statement(false, false);
            if let Some(s) = s {
                if matches!(s, Statement::If { .. } | Statement::Block { .. }) {
                    Some(s)
                } else {
                    self.errors.push(CompileError {
                        message: "Can only have if statement or block after else".into(),
                        line: if_end,
                    });
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        Ok(Statement::If {
            condition,
            block,
            else_stmt: else_stmt.map(Box::new),
            if_end,
        })
    }

    fn while_loop(&mut self) -> CompileResult<Statement> {
        let condition = self.expression()?;
        self.ignore_newline();
        self.consume(
            TokenType::LeftBrace,
            "Expect { after condition in while statement".into(),
        )?;
        let block = self.block()?;
        let end_line = self.previous.line;
        Ok(Statement::While {
            condition,
            block,
            end_line,
        })
    }

    fn for_loop(&mut self) -> CompileResult<Statement> {
        let begin_line = self.previous.line;
        self.consume(TokenType::Identifier, "Expect identifier after for".into())?;
        let iter = self.previous.inner.to_string();
        self.consume(TokenType::In, "Expect in after loop variable".into())?;
        let start = self.expression()?;
        self.consume(TokenType::DotDot, "Expect ..".into())?;
        let end = self.expression()?;
        self.ignore_newline();
        self.consume(
            TokenType::LeftBrace,
            "Expect { after range in for statement".into(),
        )?;
        let block = self.block()?;
        let end_line = self.previous.line;
        Ok(Statement::For {
            begin_line,
            iter,
            start: Box::new(start),
            end: Box::new(end),
            block,
            end_line,
        })
    }

    fn closure(&mut self) -> CompileResult<Expr> {
        let line = self.previous.line;
        let mut args: Vec<String> = vec![];
        loop {
            if self.current.token_type == TokenType::Pipe {
                self.advance();
                break;
            }
            self.consume(TokenType::Identifier, "Expect argument name".into())?;
            args.push(self.previous.inner.to_string());
            if self.match_token(TokenType::Pipe) {
                break;
            }
            self.consume(TokenType::Comma, "Expect comma after argument".into())?;
        }
        if self.match_token(TokenType::LeftBrace) {
            let block = self.block()?;
            let last_line = self.previous.line;
            Ok(Expr::Closure {
                line,
                args,
                body: ClosureBody::Block(block),
                last_line,
            })
        } else {
            let expr = self.expression()?;
            let last_line = self.previous.line;
            Ok(Expr::Closure {
                line,
                args,
                body: ClosureBody::Expr(Box::new(expr)),
                last_line,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::Scanner;
    #[test]
    fn test() {
        let tests: Vec<String> =
            serde_json::from_str(include_str!("../tests/parser_tests/tests.json")).unwrap();
        for test in tests {
            let s = std::fs::read_to_string(format!("tests/parser_tests/{}.np", test)).unwrap();
            let s = Scanner::new(&s);
            let tokens = s.scan_tokens();
            let parser = Parser::new(tokens.into_iter());
            let (stmts, errors) = parser.parse(test == "test_map_eval");
            assert!(errors.is_empty());
            if std::env::var("GENERATE_TESTS").is_ok() {
                std::fs::write(
                    format!("tests/parser_tests/{}.json", test),
                    serde_json::to_string_pretty(&stmts).unwrap(),
                )
                .unwrap();
            } else {
                let expected =
                    std::fs::read_to_string(format!("tests/parser_tests/{}.json", test)).unwrap();
                assert_eq!(expected, serde_json::to_string_pretty(&stmts).unwrap());
            }
        }
    }

    #[test]
    fn error() {
        let tests: Vec<String> =
            serde_json::from_str(include_str!("../tests/parser_tests/errors.json")).unwrap();
        for test in tests {
            let s = std::fs::read_to_string(format!("tests/parser_tests/{}.np", test)).unwrap();
            let s = Scanner::new(&s);
            let tokens = s.scan_tokens();
            let parser = Parser::new(tokens.into_iter());
            let (_, errors) = parser.parse(false);
            if std::env::var("GENERATE_TESTS").is_ok() {
                std::fs::write(
                    format!("tests/parser_tests/{}.json", test),
                    serde_json::to_string_pretty(&errors).unwrap(),
                )
                .unwrap();
            } else {
                let expected =
                    std::fs::read_to_string(format!("tests/parser_tests/{}.json", test)).unwrap();
                assert_eq!(expected, serde_json::to_string_pretty(&errors).unwrap());
            }
        }
    }
}
