#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum Type {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Variable(String),
    ArrayElement(String, Box<Expression>),
    HexLiteral(u64, bool), // (value, is_negative)
    FunctionCall(String, Vec<Expression>),
    UnaryOp(UnaryOperator, Box<Expression>),
    BinaryOp(BinaryOperator, Box<Expression>, Box<Expression>),
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum UnaryOperator {
    BitwiseNot, // ~
    Exists,     // ?
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum BinaryOperator {
    ShiftLeft,   // <<
    ShiftRight,  // >>
    BitwiseAnd,  // &
    BitwiseOr,   // |
    Add,         // +
    Sub,         // -
    LessThan,    // <
    GreaterThan, // >
    Equal,       // = (Comparison)
    Assign,      // = (Assignment)
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    VariableDeclaration {
        ty: Type,
        name: String,
        initializer: Option<Expression>,
    },
    ArrayDeclaration {
        ty: Type,
        size: ArraySize,
        name: String,
        initializer: Option<Vec<Expression>>,
    },
    FunctionDeclaration {
        ret_type: Type,
        name: String,
        params: Vec<(Type, String)>,
        body: Vec<Statement>,
    },
    Expression(Expression),
    Loop(Vec<Statement>),
    TryCatch {
        try_body: Vec<Statement>,
        catch_clauses: Vec<CatchClause>,
    },
    Throw {
        exception_code: u64,
        condition: Expression,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ArraySize {
    Variable(String),
    Literal(u64),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CatchClause {
    pub exception_code: u64,
    pub body: Vec<Statement>,
}

pub type Program = Vec<Statement>;
