use crate::ast::*;
use crate::lexer::Token;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ParseError {
    #[error("Unexpected token: expected {expected}, found {found:?}")]
    UnexpectedToken { expected: String, found: Option<Token> },
    #[error("Unexpected end of input")]
    UnexpectedEndOfInput,
    #[error("Invalid assignment target")]
    InvalidAssignmentTarget,
    #[error("Syntax error: {0}")]
    SyntaxError(String),
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<&Token> {
        let token = self.tokens.get(self.pos);
        self.pos += 1;
        token
    }

    fn consume(&mut self, expected: &Token) -> Result<(), ParseError> {
        match self.peek() {
            Some(t) if t == expected => {
                self.advance();
                Ok(())
            }
            Some(t) => Err(ParseError::UnexpectedToken {
                expected: format!("{:?}", expected),
                found: Some(t.clone()),
            }),
            None => Err(ParseError::UnexpectedEndOfInput),
        }
    }

    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut program = Vec::new();
        while self.peek().is_some() {
            program.push(self.parse_statement()?);
        }
        Ok(program)
    }

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        let stmt = match self.peek() {
            Some(Token::Var) => self.parse_var_decl()?,
            Some(Token::Array) => self.parse_array_decl()?,
            Some(Token::Func) => self.parse_func_decl()?,
            Some(Token::Loop) => self.parse_loop_stmt()?,
            Some(Token::Try) => self.parse_try_catch_stmt()?,
            Some(Token::Throw) => self.parse_throw_stmt()?,
            _ => Statement::Expression(self.parse_assignment_expression()?),
        };

        self.consume(&Token::Dot)?;
        Ok(stmt)
    }

    fn parse_var_decl(&mut self) -> Result<Statement, ParseError> {
        self.consume(&Token::Var)?;
        let ty = self.parse_type()?;
        let name = match self.advance() {
            Some(Token::Variable(n)) => n.clone(),
            found => return Err(ParseError::UnexpectedToken { expected: "Variable".to_string(), found: found.cloned() }),
        };

        let initializer = if let Some(Token::Equal) = self.peek() {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };
        Ok(Statement::VariableDeclaration { ty, name, initializer })
    }

    fn parse_array_decl(&mut self) -> Result<Statement, ParseError> {
        self.consume(&Token::Array)?;
        let ty = self.parse_type()?;
        self.consume(&Token::Asterisk)?;

        let size = match self.advance() {
            Some(Token::Variable(n)) => ArraySize::Variable(n.clone()),
            Some(Token::HexLiteral(v, _neg)) => ArraySize::Literal(*v), // Array size cannot be negative realistically but we ignore neg sign check here for simplicity or handle it if needed
            found => return Err(ParseError::UnexpectedToken { expected: "Variable or HexLiteral".to_string(), found: found.cloned() }),
        };

        let name = match self.advance() {
            Some(Token::Variable(n)) => n.clone(),
            found => return Err(ParseError::UnexpectedToken { expected: "ArrayIdentifier".to_string(), found: found.cloned() }),
        };

        let initializer = if let Some(Token::Equal) = self.peek() {
            self.advance();
            self.consume(&Token::LBrace)?;
            let mut init_list = Vec::new();
            if self.peek() != Some(&Token::RBrace) {
                init_list.push(self.parse_expression()?);
                while self.peek() == Some(&Token::Comma) {
                    self.advance();
                    init_list.push(self.parse_expression()?);
                }
            }
            self.consume(&Token::RBrace)?;
            Some(init_list)
        } else {
            None
        };

        Ok(Statement::ArrayDeclaration { ty, size, name, initializer })
    }

    fn parse_func_decl(&mut self) -> Result<Statement, ParseError> {
        self.consume(&Token::Func)?;
        let ret_type = self.parse_type()?;
        let name = match self.advance() {
            Some(Token::Function(n)) => n.clone(),
            found => return Err(ParseError::UnexpectedToken { expected: "FunctionIdentifier".to_string(), found: found.cloned() }),
        };

        let mut params = Vec::new();
        if self.peek() == Some(&Token::LParen) {
            self.advance();
            if self.peek() != Some(&Token::RParen) {
                let p_type = self.parse_type()?;
                let p_name = match self.advance() {
                    Some(Token::Variable(n)) => n.clone(),
                    found => return Err(ParseError::UnexpectedToken { expected: "Variable".to_string(), found: found.cloned() }),
                };
                params.push((p_type, p_name));

                while self.peek() == Some(&Token::Comma) {
                    self.advance();
                    let p_type = self.parse_type()?;
                    let p_name = match self.advance() {
                        Some(Token::Variable(n)) => n.clone(),
                        found => return Err(ParseError::UnexpectedToken { expected: "Variable".to_string(), found: found.cloned() }),
                    };
                    params.push((p_type, p_name));
                }
            }
            self.consume(&Token::RParen)?;
        }

        self.consume(&Token::Equal)?;
        self.consume(&Token::LBrace)?;

        let mut body = Vec::new();
        while self.peek() != Some(&Token::RBrace) && self.peek().is_some() {
            body.push(self.parse_statement()?);
        }
        self.consume(&Token::RBrace)?;

        Ok(Statement::FunctionDeclaration { ret_type, name, params, body })
    }

    fn parse_loop_stmt(&mut self) -> Result<Statement, ParseError> {
        self.consume(&Token::Loop)?;
        self.consume(&Token::LBrace)?;
        let mut body = Vec::new();
        while self.peek() != Some(&Token::RBrace) && self.peek().is_some() {
            body.push(self.parse_statement()?);
        }
        self.consume(&Token::RBrace)?;
        Ok(Statement::Loop(body))
    }

    fn parse_try_catch_stmt(&mut self) -> Result<Statement, ParseError> {
        self.consume(&Token::Try)?;
        self.consume(&Token::LBrace)?;
        let mut try_body = Vec::new();
        while self.peek() != Some(&Token::RBrace) && self.peek().is_some() {
            try_body.push(self.parse_statement()?);
        }
        self.consume(&Token::RBrace)?;

        let mut catch_clauses = Vec::new();
        while self.peek() == Some(&Token::Catch) {
            self.advance();
            let exception_code = match self.advance() {
                Some(Token::HexLiteral(v, _neg)) => *v,
                found => return Err(ParseError::UnexpectedToken { expected: "HexLiteral".to_string(), found: found.cloned() }),
            };
            self.consume(&Token::LBrace)?;
            let mut body = Vec::new();
            while self.peek() != Some(&Token::RBrace) && self.peek().is_some() {
                body.push(self.parse_statement()?);
            }
            self.consume(&Token::RBrace)?;
            catch_clauses.push(CatchClause { exception_code, body });
        }

        Ok(Statement::TryCatch { try_body, catch_clauses })
    }

    fn parse_throw_stmt(&mut self) -> Result<Statement, ParseError> {
        self.consume(&Token::Throw)?;
        let exception_code = match self.advance() {
            Some(Token::HexLiteral(v, _neg)) => *v,
            found => return Err(ParseError::UnexpectedToken { expected: "HexLiteral".to_string(), found: found.cloned() }),
        };
        self.consume(&Token::At)?;
        let condition = self.parse_expression()?;
        Ok(Statement::Throw { exception_code, condition })
    }

    fn parse_type(&mut self) -> Result<Type, ParseError> {
        match self.advance() {
            Some(Token::Type(ty)) => Ok(*ty),
            found => Err(ParseError::UnexpectedToken { expected: "Type".to_string(), found: found.cloned() }),
        }
    }

    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        self.parse_comparison_expression()
    }

    fn parse_assignment_expression(&mut self) -> Result<Expression, ParseError> {
        let lhs = self.parse_primary_expression()?;

        if self.peek() == Some(&Token::Equal) {
            self.advance(); // consume '='
            let rhs = self.parse_comparison_expression()?;

            match &lhs {
                Expression::Variable(_) | Expression::ArrayElement(_, _) => {
                    return Ok(Expression::BinaryOp(BinaryOperator::Assign, Box::new(lhs), Box::new(rhs)));
                }
                _ => return Err(ParseError::InvalidAssignmentTarget),
            }
        }

        Err(ParseError::SyntaxError(format!("Expression statement must be an assignment, found {:?}", self.peek())))
    }

    fn parse_comparison_expression(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_additive_expression()?;

        if let Some(op) = self.peek() {
            let bin_op = match op {
                Token::LessThan => Some(BinaryOperator::LessThan),
                Token::GreaterThan => Some(BinaryOperator::GreaterThan),
                Token::Equal => Some(BinaryOperator::Equal),
                _ => None,
            };

            if let Some(o) = bin_op {
                self.advance();
                let rhs = self.parse_additive_expression()?;
                expr = Expression::BinaryOp(o, Box::new(expr), Box::new(rhs));
            }
        }

        Ok(expr)
    }

    fn parse_additive_expression(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_shift_expression()?;

        while let Some(op) = self.peek() {
            let bin_op = match op {
                Token::Plus => BinaryOperator::Add,
                Token::Minus => BinaryOperator::Sub,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_shift_expression()?;
            expr = Expression::BinaryOp(bin_op, Box::new(expr), Box::new(rhs));
        }

        Ok(expr)
    }

    fn parse_shift_expression(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_bitwise_and_expression()?;

        while let Some(op) = self.peek() {
            let bin_op = match op {
                Token::ShiftLeft => BinaryOperator::ShiftLeft,
                Token::ShiftRight => BinaryOperator::ShiftRight,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_bitwise_and_expression()?;
            expr = Expression::BinaryOp(bin_op, Box::new(expr), Box::new(rhs));
        }

        Ok(expr)
    }

    fn parse_bitwise_and_expression(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_bitwise_or_expression()?;

        while self.peek() == Some(&Token::BitwiseAnd) {
            self.advance();
            let rhs = self.parse_bitwise_or_expression()?;
            expr = Expression::BinaryOp(BinaryOperator::BitwiseAnd, Box::new(expr), Box::new(rhs));
        }

        Ok(expr)
    }

    fn parse_bitwise_or_expression(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_unary_expression()?;

        while self.peek() == Some(&Token::BitwiseOr) {
            self.advance();
            let rhs = self.parse_unary_expression()?;
            expr = Expression::BinaryOp(BinaryOperator::BitwiseOr, Box::new(expr), Box::new(rhs));
        }

        Ok(expr)
    }

    fn parse_unary_expression(&mut self) -> Result<Expression, ParseError> {
        let expr = self.parse_primary_expression()?;

        if let Some(op) = self.peek() {
            let un_op = match op {
                Token::BitwiseNot => Some(UnaryOperator::BitwiseNot),
                Token::Exists => Some(UnaryOperator::Exists),
                _ => None,
            };
            if let Some(o) = un_op {
                self.advance();
                return Ok(Expression::UnaryOp(o, Box::new(expr)));
            }
        }

        Ok(expr)
    }

    fn parse_primary_expression(&mut self) -> Result<Expression, ParseError> {
        match self.peek() {
            Some(Token::HexLiteral(v, neg)) => {
                let val = *v;
                let is_neg = *neg;
                self.advance();
                Ok(Expression::HexLiteral(val, is_neg))
            }
            Some(Token::LParen) => {
                self.advance();
                let expr = self.parse_expression()?;
                self.consume(&Token::RParen)?;
                Ok(expr)
            }
            Some(Token::Variable(name)) => {
                let name_clone = name.clone();
                self.advance();
                if self.peek() == Some(&Token::LBracket) {
                    self.advance();
                    let index_expr = self.parse_expression()?;
                    self.consume(&Token::RBracket)?;
                    Ok(Expression::ArrayElement(name_clone, Box::new(index_expr)))
                } else {
                    Ok(Expression::Variable(name_clone))
                }
            }
            Some(Token::Function(name)) => {
                let name_clone = name.clone();
                self.advance();
                let mut args = Vec::new();
                if self.peek() == Some(&Token::LParen) {
                    self.advance();
                    if self.peek() != Some(&Token::RParen) {
                        args.push(self.parse_expression()?);
                        while self.peek() == Some(&Token::Comma) {
                            self.advance();
                            args.push(self.parse_expression()?);
                        }
                    }
                    self.consume(&Token::RParen)?;
                }
                Ok(Expression::FunctionCall(name_clone, args))
            }
            found => Err(ParseError::UnexpectedToken { expected: "Primary Expression".to_string(), found: found.cloned() }),
        }
    }
}
