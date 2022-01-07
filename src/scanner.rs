// This file contains the scanner which does lexing and automatic statement separator insertion
use phf::phf_map;
use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;

pub struct Scanner<'src> {
    source: &'src str,
    tokens: Vec<Token<'src>>,
    start: usize,   //Start of the current token being scanned
    current: usize, //Current end of the current token being scanned
    line: u32,
    brackets: Vec<TokenType>, // keeps track of all brackets within which the scanner is currently nested
    delims: Vec<u8>,          //string delimiters
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum TokenType {
    LeftParen,
    RightParen,
    LeftSquareBracket,
    RightSquareBracket,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    DotDot,
    Minus,
    Mod,
    ModEqual,
    Plus,
    StatementSeparator,
    Slash,
    Star,
    Colon,
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    PlusEqual,
    MinusEqual,
    StarEqual,
    SlashEqual,
    Tilde,
    TildeEqual,
    EqualEqualEqual,
    BangEqualEqual,
    Pipe,
    // Literals.
    Identifier,
    String(String),
    IntLiteral(i32),
    FloatLiteral(f64),
    Symbol(String),
    // Keywords.
    And,
    Break,
    Class,
    Const,
    Continue,
    Else,
    Export,
    Extends,
    False,
    For,
    Fun,
    If,
    In,
    New,
    Null,
    Or,
    Return,
    Super,
    This,
    True,
    Let,
    While,
    Try,
    Catch,
    Throw,
    Map,
    //Other types
    Interpolation, // It stores
    Error(String),
    Eof,
}

static KEYWORDS: phf::Map<&'static str, TokenType> = phf_map! {
    "and" => TokenType::And,
    "break" => TokenType::Break,
    "class" => TokenType::Class,
    "continue" => TokenType::Continue,
    "const" => TokenType::Const,
    "else" => TokenType::Else,
    "export" => TokenType::Export,
    "extends" => TokenType::Extends,
    "false" => TokenType::False,
    "for" => TokenType::For,
    "fun" => TokenType::Fun,
    "if" => TokenType::If,
    "new" => TokenType::New,
    "null" => TokenType::Null,
    "or" => TokenType::Or,
    "return" => TokenType::Return,
    "super" => TokenType::Super,
    "this" => TokenType::This,
    "true" => TokenType::True,
    "let" => TokenType::Let,
    "while" => TokenType::While,
    "try" => TokenType::Try,
    "throw" => TokenType::Throw,
    "catch" => TokenType::Catch,
    "in" => TokenType::In,
    "Map"=> TokenType::Map,
};

//Returns corresponding keyword tokens for string
fn get_keyword(s: &str) -> Option<TokenType> {
    KEYWORDS.get(s).cloned()
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Token<'src> {
    pub token_type: TokenType,
    pub inner: &'src str,
    pub line: u32,
}

impl<'src> Token<'src> {
    pub fn uninit_token() -> Token<'static> {
        Token {
            token_type: TokenType::Eof,
            inner: "",
            line: 0,
        }
    }
}

impl<'src> Scanner<'src> {
    pub fn new(source: &'src str) -> Self {
        Self {
            source,
            tokens: vec![],
            start: 0,
            current: 0,
            line: 1,
            brackets: vec![],
            delims: vec![],
        }
    }

    pub fn scan_tokens(mut self) -> Vec<Token<'src>> {
        while self.scan_token() {
            self.start = self.current;
        }
        self.tokens.push(Token {
            token_type: TokenType::Eof,
            inner: "",
            line: self.line,
        });
        self.tokens
    }

    //Scans one or more tokens returns false if it reaches end of file
    fn scan_token(&mut self) -> bool {
        let c = match self.advance() {
            Some(c) => c,
            None => return false,
        };
        match c {
            b'\0' => self.error("Nul character in file".to_string()),
            b'@' => self.symbol(),
            b':' => self.add_token(TokenType::Colon),
            b'(' => {
                self.add_token(TokenType::LeftParen);
                self.brackets.push(TokenType::LeftParen);
            }
            b')' => {
                self.add_token(TokenType::RightParen);
                self.brackets.pop();
                if self.brackets.last().cloned() == Some(TokenType::Interpolation) {
                    self.brackets.pop();
                    let last_delim = self.delims.pop().unwrap();
                    self.string(last_delim);
                }
            }
            b'[' => {
                self.add_token(TokenType::LeftSquareBracket);
                self.brackets.push(TokenType::LeftSquareBracket);
            }
            b']' => {
                self.add_token(TokenType::RightSquareBracket);
                self.brackets.pop();
            }
            b'{' => {
                self.add_token(TokenType::LeftBrace);
                self.brackets.push(TokenType::LeftBrace);
            }
            b'}' => {
                self.add_token(TokenType::RightBrace);
                self.brackets.pop();
            }
            b',' => self.add_token(TokenType::Comma),
            b'.' => self.add_token_if_match(b'.', TokenType::DotDot, TokenType::Dot),
            b'-' => self.add_token_if_match(b'=', TokenType::MinusEqual, TokenType::Minus),
            b'+' => self.add_token_if_match(b'=', TokenType::PlusEqual, TokenType::Plus),
            b';' => self.add_token(TokenType::StatementSeparator),
            b'*' => self.add_token_if_match(b'=', TokenType::StarEqual, TokenType::Star),
            b'%' => self.add_token_if_match(b'=', TokenType::ModEqual, TokenType::Mod),
            b'!' => {
                if self.match_char(b'=') {
                    if self.match_char(b'=') {
                        self.add_token(TokenType::BangEqualEqual)
                    } else {
                        self.add_token(TokenType::BangEqual)
                    }
                } else {
                    self.add_token(TokenType::Bang);
                }
            }
            b'=' => {
                if self.match_char(b'=') {
                    if self.match_char(b'=') {
                        self.add_token(TokenType::EqualEqualEqual)
                    } else {
                        self.add_token(TokenType::EqualEqual)
                    }
                } else {
                    self.add_token(TokenType::Equal);
                }
            }
            b'>' => self.add_token_if_match(b'=', TokenType::GreaterEqual, TokenType::Greater),
            b'<' => self.add_token_if_match(b'=', TokenType::LessEqual, TokenType::Less),
            b'/' => {
                if self.match_char(b'=') {
                    self.add_token(TokenType::SlashEqual);
                }
                //Single line comment
                else if self.match_char(b'/') {
                    while self.peek() != Some(b'\n') && self.peek() != None {
                        self.advance();
                    }
                }
                //Multiline comment
                else if self.match_char(b'*') {
                    let mut depth = 1;
                    while depth != 0 && self.peek() != None {
                        if self.peek() == Some(b'\n') {
                            self.line += 1;
                        }
                        if self.peek() == Some(b'/') && self.peek_next() == Some(b'*') {
                            depth += 1;
                            self.advance();
                        } else if self.peek() == Some(b'*') && self.peek_next() == Some(b'/') {
                            depth -= 1;
                            self.advance();
                        }
                        self.advance();
                    }
                    if depth != 0 {
                        self.error("Unterminated multiline comment".to_string());
                    }
                } else {
                    self.add_token(TokenType::Slash)
                }
            }
            b' ' | b'\r' | b'\t' => {}
            b'\n' => {
                self.line += 1;
                /*
                Automatic Statement Seperator Insertion
                When newline is encountered if its previous token is RightParen|RightBrace|Identifier
                |IntLiteral|FloatLiteral|False|Null|Return|Super|This|True|EndString|Symbol
                and it is not followed by whitespaces and a dot (to allow method chaining) and it is not inside brackets(except block) then insert semicolon.

                Statements like if and for can have a newline after the condition
                */
                if self.brackets.is_empty()
                    || matches!(self.brackets.last(), Some(TokenType::LeftBrace))
                {
                    if let Some(token) = self.tokens.last() {
                        match token.token_type {
                            TokenType::RightParen
                            | TokenType::RightBrace
                            | TokenType::RightSquareBracket
                            | TokenType::Identifier
                            | TokenType::IntLiteral(_)
                            | TokenType::FloatLiteral(_)
                            | TokenType::False
                            | TokenType::Null
                            | TokenType::Return
                            | TokenType::Super
                            | TokenType::This
                            | TokenType::True
                            | TokenType::String(_)
                            | TokenType::Symbol(_)
                            | TokenType::Break
                            | TokenType::Continue => {
                                let l = self.line - 1;
                                self.consume_whitespaces();
                                if self.peek() != Some(b'.') {
                                    self.tokens.push(Token {
                                        token_type: TokenType::StatementSeparator,
                                        inner: "\n",
                                        line: l,
                                    })
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            b'"' => self.string(b'"'),
            b'\'' => self.string(b'\''),
            b'~' => self.add_token_if_match(b'=', TokenType::TildeEqual, TokenType::Tilde),
            b'|' => self.add_token(TokenType::Pipe),
            c => {
                if isdigit(c) {
                    self.number(c);
                } else if isalpha(c) {
                    self.identifier();
                } else {
                    let c = (&self.source[(self.current - 1)..])
                        .graphemes(true)
                        .next()
                        .unwrap();
                    let len = c.len();
                    self.error(format!("Unexpected character '{}'", c));
                    self.current += len - 1;
                }
            }
        };
        true
    }

    fn string(&mut self, delim: u8) {
        /*
        Strings can have normal characters or \n,\t,\r,\0,\',\",\\,\u{unicode excape sequence} and interpolation.
        For Example, the string "abc\(d)efg\(h)ij"  will generate
        String(abc) Interpolation LeftParen Identifier(d) RightParen String(efg) Interpolation LeftParen Identifier(h) RightParen String(ij)
        */
        self.start += 1;
        let mut s: Vec<u8> = vec![];
        while self.peek() != Some(delim) && self.peek() != None {
            if self.peek() == Some(b'\n') {
                self.line += 1;
                s.push(b'\n');
                self.advance();
            }
            //Interpolation
            else if self.peek() == Some(b'\\') && self.peek_next() == Some(b'(') {
                self.add_token(TokenType::String(String::from_utf8(s).unwrap()));
                self.start = self.current;
                self.advance();
                self.add_token(TokenType::Interpolation);
                self.delims.push(delim);
                self.brackets.push(TokenType::Interpolation);
                return;
            } else if self.peek() == Some(b'\\') && self.peek_next() == Some(b'u') {
                self.advance();
                self.advance();
                if self.match_char(b'{') {
                    let start = self.current;
                    while self.peek() != None
                        && self.peek() != Some(delim)
                        && self.peek() != Some(b'}')
                    {
                        self.advance();
                    }
                    if self.peek() == Some(delim) || self.peek() == None {
                        self.error("Unterminated unicode escape sequence".to_string());
                    } else {
                        match parse_int::parse::<u32>(&format!(
                            "0x{}",
                            &self.source[start..self.current]
                        )) {
                            Ok(n) => {
                                if let Some(c) = char::from_u32(n) {
                                    for byte in
                                        char::from_u32(c as u32).unwrap().to_string().bytes()
                                    {
                                        s.push(byte);
                                    }
                                } else {
                                    self.error(format!(
                                        "Cannot convert {} to a unicode character",
                                        &self.source[start..self.current]
                                    ));
                                }
                            }
                            Err(_) => self.error(format!(
                                "Cannot parse {} in unicode escape sequence",
                                &self.source[start..self.current]
                            )),
                        }
                        self.advance();
                    }
                } else {
                    self.error("Unicode character not given".to_string());
                }
            }
            //Escape sequence
            else if self.peek() == Some(b'\\') {
                if let Some(c) = self.peek_next() {
                    let to_add = match c {
                        b'\\' => b'\\',
                        b'n' => b'\n',
                        b'r' => b'\r',
                        b'\'' => b'\'',
                        b'"' => b'"',
                        b'0' => b'\0',
                        b't' => b'\t',
                        c => {
                            self.error(format!("Invalid escape sequence \\{}", c as char));
                            b' '
                        }
                    };
                    s.push(to_add);
                } else {
                    self.error("Unterminated \\".to_string());
                }
                self.advance();
                if self.peek() != None {
                    self.advance();
                }
            } else {
                s.push(self.advance().unwrap());
            }
        }
        self.add_token(TokenType::String(String::from_utf8(s).unwrap()));
        if self.peek() != None {
            self.start = self.current;
            self.advance();
        }
    }

    //A number can be either decimal,octal or hexadecimal
    //A octal number starts wiyh 0o
    //A hexadecimal number starts with 0x
    //A number cannot start with 0
    //Only decimal numbers can be floating point
    //Floating point numbers can be of the form {x}.{y} or {x}e{y} or {x}.{y}e{z}
    //This function checks if there is .. whenever it scans . in order to stop scanning
    //and make it a range
    fn number(&mut self, c: u8) {
        let mut errors = vec![];
        let mut is_float = false;
        let mut can_float = true;
        let mut handled_e = false;
        let mut is_range = false;
        let mut is_hex = false;
        if c == b'0' {
            match self.peek() {
                Some(b'x') => {
                    can_float = false;
                    is_hex = true;
                    self.advance();
                }
                Some(b'o') => {
                    can_float = false;
                    self.advance();
                }
                Some(b'0'..=b'9') => {
                    errors.push("Use 0o for octal".to_string());
                }
                _ => {}
            }
        }
        while self
            .peek()
            .map_or(false, |c| is_valid_num_char(c, is_hex) || c == b'_')
        {
            self.advance();
        }
        if self.peek() == Some(b'.') {
            if self.peek_next() == Some(b'.') {
                is_range = true;
            } else if self
                .peek_next()
                .map_or(false, |c| is_valid_num_char(c, false))
            {
                if !can_float {
                    errors.push("Can use floating numbers only in decimals".to_string());
                    self.advance();
                }
                self.advance();
                is_float = true;
            } else if self.peek_next() == None {
                errors.push("Expect number after .".to_string());
            }
        } else if self.peek() == Some(b'e') {
            if self.peek_next() == Some(b'+') || self.peek_next() == Some(b'-') {
                self.advance();
            }
            handled_e = true;
            if !can_float {
                errors.push("Can use floating numbers only in decimals".to_string());
                self.advance();
            } else if self
                .peek_next()
                .map_or(false, |c| is_valid_num_char(c, is_hex))
            {
                self.advance();
                is_float = true;
            } else if self.peek_next() == None {
                errors.push("Expect number after e".to_string());
            }
        }
        while self
            .peek()
            .map_or(false, |c| is_valid_num_char(c, is_hex) || c == b'_')
        {
            self.advance();
        }
        if self.peek() == Some(b'e') && !handled_e && !is_range {
            if self.peek_next() == Some(b'+') || self.peek_next() == Some(b'-') {
                self.advance();
            }
            if !can_float {
                errors.push("Can use floating numbers only in decimals".to_string());
                self.advance();
            } else if self
                .peek_next()
                .map_or(false, |c| is_valid_num_char(c, is_hex))
            {
                self.advance();
            } else if self.peek_next() == None {
                errors.push("Expect number after e".to_string());
            }
        }
        while self
            .peek()
            .map_or(false, |c| is_valid_num_char(c, is_hex) || c == b'_')
        {
            self.advance();
        }
        let string = &self.source[self.start..self.current];
        if is_float {
            let token = if errors.is_empty() {
                match parse_int::parse(string) {
                    Ok(f) => f,
                    Err(_) => {
                        self.error(format!("Cannot parse float {}", string));
                        0.0
                    }
                }
            } else {
                for i in errors {
                    self.error(format!("{} for {}", i, string));
                }
                0.0
            };
            self.add_token(TokenType::FloatLiteral(token));
        } else {
            let token = if errors.is_empty() {
                match parse_int::parse::<u32>(string) {
                    // -1 is a sentinel value used to denote 2147483648
                    // This number needs special treatment as -2147483648 
                    // is a valid literal but 2147483648 isnt
                    Ok(f) => match f {
                        0..=2147483647 => f as i32,
                        2147483648 => -1,
                        _ => {
                            self.error(format!("Cannot parse integer {}", string));
                            0
                        }
                    },
                    Err(_) => {
                        self.error(format!("Cannot parse integer {}", string));
                        0
                    }
                }
            } else {
                for i in errors {
                    self.error(format!("{} for {}", i, string));
                }
                0
            };
            self.add_token(TokenType::IntLiteral(token));
        }
    }

    fn identifier(&mut self) {
        while self.peek().map_or(false, isalnum) {
            self.advance();
        }
        let ttype = match get_keyword(&self.source[self.start..self.current]) {
            Some(t) => t,
            None => TokenType::Identifier,
        };
        self.add_token(ttype);
    }

    fn symbol(&mut self) {
        if !self.peek().map_or(false, isalpha) {
            match self.peek() {
                Some(c) => {
                    self.error(format!("Invalid character {} after @ in symbol", c as char));
                }
                None => self.error("Unexpected end of file after @".to_string()),
            }
        }
        while self.peek().map_or(false, isalnum) {
            self.advance();
        }
        self.add_token(TokenType::Symbol(
            self.source[self.start + 1..self.current].into(),
        ));
    }

    fn match_char(&mut self, expected: u8) -> bool {
        if self.peek() == None || self.source.as_bytes()[self.current] != expected {
            false
        } else {
            self.current += 1;
            true
        }
    }

    fn add_token_if_match(&mut self, expected: u8, if_match: TokenType, if_not_match: TokenType) {
        if self.match_char(expected) {
            self.add_token(if_match)
        } else {
            self.add_token(if_not_match)
        }
    }
    //Increments current and returns the previous character
    fn advance(&mut self) -> Option<u8> {
        let c = self.peek();
        self.current += 1;
        c
    }

    //Gives the current character
    fn peek(&self) -> Option<u8> {
        if self.current >= self.source.len() {
            None
        } else {
            Some(self.source.as_bytes()[self.current])
        }
    }
    //Gives the character after the current character
    fn peek_next(&self) -> Option<u8> {
        if self.current + 1 >= self.source.len() {
            None
        } else {
            Some(self.source.as_bytes()[self.current + 1])
        }
    }

    fn consume_whitespaces(&mut self) {
        loop {
            match self.peek() {
                Some(b' ') | Some(b'\t') | Some(b'\r') => {
                    self.advance();
                }
                Some(b'\n') => {
                    self.line += 1;
                    self.advance();
                }
                _ => return,
            }
        }
    }

    fn add_token(&mut self, token_type: TokenType) {
        self.tokens.push(Token {
            token_type,
            inner: &self.source[self.start..self.current],
            line: self.line,
        })
    }

    fn error(&mut self, message: String) {
        self.tokens.push(Token {
            token_type: TokenType::Error(message),
            inner: "",
            line: self.line,
        });
    }
}

fn is_valid_num_char(c: u8, is_hex: bool) -> bool {
    if is_hex {
        isalnum(c)
    } else {
        isdigit(c)
    }
}

fn isalpha(c: u8) -> bool {
    (b'a'..=b'z').contains(&c) || (b'A'..=b'Z').contains(&c) || c == b'_'
}

fn isdigit(c: u8) -> bool {
    (b'0'..=b'9').contains(&c)
}

fn isalnum(c: u8) -> bool {
    isalpha(c) || isdigit(c)
}
