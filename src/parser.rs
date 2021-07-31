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
        TokenType::Final => Precedence::None,
        TokenType::While => Precedence::None,
        TokenType::Interpolation => Precedence::None,
        TokenType::Eof => Precedence::None,
        TokenType::Error(_) => Precedence::None,
        TokenType::PlusEqual => Precedence::Assignment,
        TokenType::MinusEqual => Precedence::Assignment,
        TokenType::StarEqual => Precedence::Assignment,
        TokenType::SlashEqual => Precedence::Assignment,
    }
}

#[derive(Debug, Serialize, Deserialize)]
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
    Variable {
        name: String,
        line: u32,
    },
}
#[derive(Debug, Serialize, Deserialize)]
pub enum Statement {
    Expr(Expr),
    VarDeclaration {
        name: String,
        mutability: Mutability,
        expr: Expr,
        line: u32,
    },
    Block(Vec<Statement>),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Mutability {
    Immutable,
    Mutable,
    Const,
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

    pub fn parse(mut self) -> (Vec<Statement>, Vec<CompileError>) {
        self.advance();
        let mut statements = vec![];
        while self.current.token_type != TokenType::Eof {
            if let Some(stmt) = self.statement(true) {
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
            TokenType::Identifier => Some(self.variable()),
            TokenType::String(_) => Some(todo!()),
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
            TokenType::Final => None,
            TokenType::While => None,
            TokenType::Interpolation => None,
            TokenType::Eof => None,
            TokenType::Error(_) => None,
            TokenType::PlusEqual => None,
            TokenType::MinusEqual => None,
            TokenType::StarEqual => None,
            TokenType::SlashEqual => None,
        }
    }

    fn infix(&mut self, token_type: TokenType, left: Box<Expr>) -> CompileResult<Expr> {
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
            TokenType::Final => unreachable!(),
            TokenType::While => unreachable!(),
            TokenType::Interpolation => unreachable!(),
            TokenType::Eof => unreachable!(),
            TokenType::Error(_) => unreachable!(),
            TokenType::PlusEqual => self.binary(left),
            TokenType::MinusEqual => self.binary(left),
            TokenType::StarEqual => self.binary(left),
            TokenType::SlashEqual => self.binary(left),
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
                // todo add more tokens
                TokenType::Class
                | TokenType::Fun
                | TokenType::Let
                | TokenType::Const
                | TokenType::Final
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

    fn statement(&mut self, needs_sep: bool) -> Option<Statement> {
        match (|| {
            let e = if self.match_token(TokenType::Let) {
                self.var_declaration(Mutability::Mutable)
            } else if self.match_token(TokenType::Final) {
                self.var_declaration(Mutability::Immutable)
            } else if self.match_token(TokenType::Const) {
                self.var_declaration(Mutability::Const)
            } else if self.match_token(TokenType::LeftBrace) {
                self.block()
            } else {
                self.expression_statement()
            }?;
            if needs_sep {
                self.consume(
                    TokenType::StatementSeparator,
                    "Expect newline or semicolon".into(),
                )?;
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

    fn var_declaration(&mut self, mutability: Mutability) -> CompileResult<Statement> {
        if TokenType::Identifier == self.current.token_type {
            self.advance();
            let name = self.previous.inner.into();
            let line = self.previous.line;
            self.consume(TokenType::Equal, "Variable must be initialized".into())?;
            let expr = self.expression()?;
            Ok(Statement::VarDeclaration {
                name,
                expr,
                mutability,
                line,
            })
        } else {
            Err(self.error_at_current("Expect identifier".into()))
        }
    }

    fn variable(&mut self) -> CompileResult<Expr> {
        Ok(Expr::Variable {
            name: self.previous.inner.into(),
            line: self.previous.line,
        })
    }

    fn block(&mut self) -> CompileResult<Statement> {
        let mut statements = vec![];
        self.consume(TokenType::LeftBrace, "Expect { to begin block".into())?;
        loop {
            if let Some(stmt) = self.statement(false) {
                statements.push(stmt);
            }
            if self.match_token(TokenType::RightBrace) {
                break;
            } else if self.match_token(TokenType::StatementSeparator) {
                if self.match_token(TokenType::RightBrace) {
                    break;
                }
            } else {
                return Err(self.error_at_current(
                    "Expect newline or semicolon or right brace after statement in block".into(),
                ));
            }
        }
        Ok(Statement::Block(statements))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::{are_brackets_balanced, Scanner};
    use std::{fs, panic::PanicInfo};
    #[test]
    fn test() {
        if std::env::var("GENERATE_TESTS").is_ok() {
            fn gen_json(name: &str) {
                let s = std::fs::read_to_string(format!("tests/parser_tests/{}.np", name)).unwrap();
                let s = Scanner::new(&s);
                let tokens = s.scan_tokens();
                let parser = Parser::new(tokens.into_iter());
                let (statements, errors) = parser.parse();
                assert!(errors.is_empty());
                std::fs::write(
                    format!("tests/parser_tests/{}.json", name),
                    serde_json::to_string(&statements).unwrap(),
                )
                .unwrap();
            }
            let tests: Vec<String> =
                serde_json::from_str(include_str!("../tests/parser_tests/tests.json")).unwrap();
            for i in tests {
                gen_json(&i);
            }
        } else {
            let tests: Vec<String> =
                serde_json::from_str(include_str!("../tests/parser_tests/tests.json")).unwrap();

            for test in tests {
                let s1 = fs::read_to_string(format!("tests/parser_tests/{}.np", test)).unwrap();
                let s2 =
                    std::fs::read_to_string(format!("tests/parser_tests/{}.json", test)).unwrap();
                let s = Scanner::new(&s1);
                let tokens = s.scan_tokens();
                let parser = Parser::new(tokens.into_iter());
                let (statements, errors) = parser.parse();
                assert!(errors.is_empty());
                assert_eq!(serde_json::to_string(&statements).unwrap(), s2);
            }
        }
    }
    #[test]
    fn error() {
        if !std::env::var("GENERATE_TESTS").is_ok() {
            let errors: Vec<String> =
                serde_json::from_str(include_str!("../tests/parser_tests/errors.json")).unwrap();
            for error in errors {
                let s = fs::read_to_string(format!("tests/parser_tests/{}.np", error)).unwrap();
                let s = Scanner::new(&s);
                let tokens = s.scan_tokens();
                let parser = Parser::new(tokens.into_iter());
                let (_, errors) = parser.parse();
                assert!(!errors.is_empty())
            }
        }
    }
}
