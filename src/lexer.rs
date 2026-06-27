use crate::ast::*;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Var,
    Array,
    Func,
    Loop,
    Try,
    Catch,
    Throw,
    Type(Type),
    Variable(String),
    Function(String),
    HexLiteral(u64, bool), // (value, is_negative)
    BitwiseNot, // ~
    Exists,     // ?
    ShiftLeft,  // <<
    ShiftRight, // >>
    BitwiseAnd, // &
    BitwiseOr,  // |
    Plus,       // +
    Minus,      // -
    LessThan,   // <
    GreaterThan,// >
    Equal,      // =
    At,         // @
    Dot,        // .
    Comma,      // ,
    Asterisk,   // *
    LBrace,     // {
    RBrace,     // }
    LParen,     // (
    RParen,     // )
    LBracket,   // [
    RBracket,   // ]
}

#[derive(Error, Debug, PartialEq)]
pub enum LexerError {
    #[error("Invalid character '{0}' at line {1}")]
    InvalidCharacter(char, usize),
    #[error("Unterminated comment at line {0}")]
    UnterminatedComment(usize),
    #[error("Invalid identifier '{0}' at line {1}")]
    InvalidIdentifier(String, usize),
    #[error("Invalid hex literal '{0}' at line {1}")]
    InvalidHexLiteral(String, usize),
}

pub struct Lexer<'a> {
    input: &'a str,
    pos: usize,
    line: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Lexer {
            input,
            pos: 0,
            line: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        if c == '\n' {
            self.line += 1;
        }
        Some(c)
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexerError> {
        let mut tokens = Vec::new();

        while self.pos < self.input.len() {
            self.skip_whitespace();
            if self.pos >= self.input.len() {
                break;
            }

            let start_pos = self.pos;
            let start_line = self.line;

            if self.peek() == Some('(') {
                // Try to tentatively lex the content inside `()`
                let mut tentative_lexer = Lexer {
                    input: self.input,
                    pos: self.pos + 1,
                    line: self.line,
                };

                let mut is_valid_expression = true;
                let mut inner_tokens = Vec::new();
                let mut found_rparen = false;

                while let Some(c) = tentative_lexer.peek() {
                    if c == ')' {
                        tentative_lexer.advance();
                        found_rparen = true;
                        break;
                    }
                    match tentative_lexer.next_token_internal() {
                        Ok(Some(token)) => {
                            inner_tokens.push(token);
                        }
                        Ok(None) => break,
                        Err(_) => {
                            is_valid_expression = false;
                            break;
                        }
                    }
                }

                if is_valid_expression && found_rparen && !inner_tokens.is_empty() {
                    // It's a valid expression or parameters inside `()`
                    self.advance(); // consume '('
                    tokens.push(Token::LParen);
                    tokens.extend(inner_tokens);
                    self.pos = tentative_lexer.pos;
                    self.line = tentative_lexer.line;
                    tokens.push(Token::RParen);
                    continue;
                } else {
                    // It's a comment.
                    self.advance(); // consume '('
                    let mut depth = 1;
                    let mut closed = false;
                    while let Some(c) = self.advance() {
                        if c == '(' {
                            depth += 1;
                        } else if c == ')' {
                            depth -= 1;
                            if depth == 0 {
                                closed = true;
                                break;
                            }
                        }
                    }
                    if !closed {
                        return Err(LexerError::UnterminatedComment(start_line));
                    }
                    continue;
                }
            }

            if let Some(token) = self.next_token_internal()? {
                tokens.push(token);
            }
        }

        Ok(tokens)
    }

    fn next_token_internal(&mut self) -> Result<Option<Token>, LexerError> {
        self.skip_whitespace();
        let c = match self.peek() {
            Some(c) => c,
            None => return Ok(None),
        };

        match c {
            '~' => { self.advance(); Ok(Some(Token::BitwiseNot)) }
            '?' => { self.advance(); Ok(Some(Token::Exists)) }
            '&' => { self.advance(); Ok(Some(Token::BitwiseAnd)) }
            '|' => { self.advance(); Ok(Some(Token::BitwiseOr)) }
            '+' => { self.advance(); Ok(Some(Token::Plus)) }
            '<' => {
                self.advance();
                if self.peek() == Some('<') {
                    self.advance();
                    Ok(Some(Token::ShiftLeft))
                } else {
                    Ok(Some(Token::LessThan))
                }
            }
            '>' => {
                self.advance();
                if self.peek() == Some('>') {
                    self.advance();
                    Ok(Some(Token::ShiftRight))
                } else {
                    Ok(Some(Token::GreaterThan))
                }
            }
            '=' => { self.advance(); Ok(Some(Token::Equal)) }
            '@' => { self.advance(); Ok(Some(Token::At)) }
            '.' => { self.advance(); Ok(Some(Token::Dot)) }
            ',' => { self.advance(); Ok(Some(Token::Comma)) }
            '*' => { self.advance(); Ok(Some(Token::Asterisk)) }
            '{' => { self.advance(); Ok(Some(Token::LBrace)) }
            '}' => { self.advance(); Ok(Some(Token::RBrace)) }
            '[' => { self.advance(); Ok(Some(Token::LBracket)) }
            ']' => { self.advance(); Ok(Some(Token::RBracket)) }
            '(' => { self.advance(); Ok(Some(Token::LParen)) }
            ')' => { self.advance(); Ok(Some(Token::RParen)) }
            '-' => {
                // Check if it's a negative hex literal
                let mut temp_lexer = Lexer { input: self.input, pos: self.pos + 1, line: self.line };
                if let Some(next_c) = temp_lexer.peek() {
                    if next_c.is_ascii_hexdigit() && next_c.is_ascii_uppercase() || next_c.is_ascii_digit() {
                        return self.lex_hex_literal();
                    }
                }
                self.advance();
                Ok(Some(Token::Minus))
            }
            '$' | '#' => self.lex_identifier(),
            _ if c.is_ascii_hexdigit() && c.is_ascii_uppercase() || c.is_ascii_digit() => self.lex_hex_literal(),
            _ if c.is_ascii_lowercase() => self.lex_keyword(),
            _ => Err(LexerError::InvalidCharacter(c, self.line)),
        }
    }

    fn lex_identifier(&mut self) -> Result<Option<Token>, LexerError> {
        let prefix = self.advance().unwrap();
        let mut name = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' {
                name.push(c);
                self.advance();
            } else {
                break;
            }
        }
        if name.is_empty() {
            return Err(LexerError::InvalidIdentifier(prefix.to_string(), self.line));
        }
        if prefix == '$' {
            Ok(Some(Token::Variable(name)))
        } else {
            Ok(Some(Token::Function(name)))
        }
    }

    fn lex_hex_literal(&mut self) -> Result<Option<Token>, LexerError> {
        let mut is_negative = false;
        if self.peek() == Some('-') {
            is_negative = true;
            self.advance();
        }

        let mut hex_str = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || (c >= 'A' && c <= 'F') {
                hex_str.push(c);
                self.advance();
            } else if c >= 'a' && c <= 'f' {
                // Lowercase hex is invalid according to spec
                return Err(LexerError::InvalidHexLiteral(hex_str + &c.to_string(), self.line));
            } else {
                break;
            }
        }

        if hex_str.is_empty() {
            return Err(LexerError::InvalidHexLiteral(if is_negative { "-".to_string() } else { "".to_string() }, self.line));
        }

        let value = u64::from_str_radix(&hex_str, 16).map_err(|_| LexerError::InvalidHexLiteral(hex_str.clone(), self.line))?;
        Ok(Some(Token::HexLiteral(value, is_negative)))
    }

    fn lex_keyword(&mut self) -> Result<Option<Token>, LexerError> {
        let mut word = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_lowercase() || c.is_ascii_digit() {
                word.push(c);
                self.advance();
            } else {
                break;
            }
        }

        match word.as_str() {
            "var" => Ok(Some(Token::Var)),
            "array" => Ok(Some(Token::Array)),
            "func" => Ok(Some(Token::Func)),
            "loop" => Ok(Some(Token::Loop)),
            "try" => Ok(Some(Token::Try)),
            "catch" => Ok(Some(Token::Catch)),
            "throw" => Ok(Some(Token::Throw)),
            "i8" => Ok(Some(Token::Type(Type::I8))),
            "u8" => Ok(Some(Token::Type(Type::U8))),
            "i16" => Ok(Some(Token::Type(Type::I16))),
            "u16" => Ok(Some(Token::Type(Type::U16))),
            "i32" => Ok(Some(Token::Type(Type::I32))),
            "u32" => Ok(Some(Token::Type(Type::U32))),
            "i64" => Ok(Some(Token::Type(Type::I64))),
            "u64" => Ok(Some(Token::Type(Type::U64))),
            _ => Err(LexerError::InvalidIdentifier(word, self.line)),
        }
    }
}
