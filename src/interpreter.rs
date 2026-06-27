use std::collections::HashMap;
use crate::ast::*;
use std::io::{Read, Write};

#[derive(Debug, Clone)]
pub enum Value {
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    Array(Type, Vec<Option<Value>>), // None means consumed
}

impl Value {
    pub fn as_u64(&self) -> u64 {
        match self {
            Value::I8(v) => *v as u64,
            Value::U8(v) => *v as u64,
            Value::I16(v) => *v as u64,
            Value::U16(v) => *v as u64,
            Value::I32(v) => *v as u64,
            Value::U32(v) => *v as u64,
            Value::I64(v) => *v as u64,
            Value::U64(v) => *v as u64,
            Value::Array(_, _) => 0,
        }
    }
    
    pub fn as_i64(&self) -> i64 {
        match self {
            Value::I8(v) => *v as i64,
            Value::U8(v) => *v as i64,
            Value::I16(v) => *v as i64,
            Value::U16(v) => *v as i64,
            Value::I32(v) => *v as i64,
            Value::U32(v) => *v as i64,
            Value::I64(v) => *v as i64,
            Value::U64(v) => *v as i64,
            Value::Array(_, _) => 0,
        }
    }

    pub fn from_u64(val: u64, ty: Type) -> Self {
        match ty {
            Type::I8 => Value::I8(val as i8),
            Type::U8 => Value::U8(val as u8),
            Type::I16 => Value::I16(val as i16),
            Type::U16 => Value::U16(val as u16),
            Type::I32 => Value::I32(val as i32),
            Type::U32 => Value::U32(val as u32),
            Type::I64 => Value::I64(val as i64),
            Type::U64 => Value::U64(val as u64),
        }
    }

    pub fn is_signed(ty: Type) -> bool {
        matches!(ty, Type::I8 | Type::I16 | Type::I32 | Type::I64)
    }

    pub fn bit_width(ty: Type) -> u8 {
        match ty {
            Type::I8 | Type::U8 => 8,
            Type::I16 | Type::U16 => 16,
            Type::I32 | Type::U32 => 32,
            Type::I64 | Type::U64 => 64,
        }
    }

    pub fn get_type(&self) -> Type {
        match self {
            Value::I8(_) => Type::I8,
            Value::U8(_) => Type::U8,
            Value::I16(_) => Type::I16,
            Value::U16(_) => Type::U16,
            Value::I32(_) => Type::I32,
            Value::U32(_) => Type::U32,
            Value::I64(_) => Type::I64,
            Value::U64(_) => Type::U64,
            Value::Array(ty, _) => *ty,
        }
    }

    pub fn default_val(ty: Type) -> Self {
        Self::from_u64(0, ty)
    }
}

pub fn check_implicit_conversion_warning(src_ty: Type, dest_ty: Type) {
    if src_ty == dest_ty {
        return;
    }
    
    let src_signed = Value::is_signed(src_ty);
    let dest_signed = Value::is_signed(dest_ty);
    let src_width = Value::bit_width(src_ty);
    let dest_width = Value::bit_width(dest_ty);

    if src_signed != dest_signed {
        eprintln!("Warning: Signed/Unsigned mismatch when converting {:?} to {:?}", src_ty, dest_ty);
    }
    if src_width > dest_width {
        eprintln!("Warning: Narrowing conversion, possible loss of data when converting {:?} to {:?}", src_ty, dest_ty);
    }
}

#[derive(Debug, Clone)]
pub struct VariableState {
    pub val: Value,
    pub is_alive: bool,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub struct Frame {
    pub scopes: Vec<HashMap<String, VariableState>>,
}

pub struct Interpreter {
    pub frames: Vec<Frame>,
    pub functions: HashMap<String, Statement>, // FunctionDeclaration
}

#[derive(Debug)]
pub enum EvalError {
    UndefinedVariable(String),
    ConsumedVariable(String),
    TypeMismatch,
    Exception(u64),
    NotImplemented(String),
    MainNotFound,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            frames: vec![Frame { scopes: vec![HashMap::new()] }],
            functions: HashMap::new(),
        }
    }

    pub fn run_program(&mut self, program: &Program) -> Result<i32, EvalError> {
        for stmt in program {
            if let Statement::FunctionDeclaration { name, .. } = stmt {
                self.functions.insert(name.clone(), stmt.clone());
            } else {
                self.eval_statement(stmt)?;
            }
        }

        // Run main if it exists
        if let Some(main_func) = self.functions.get("main").cloned() {
            let res = self.eval_function_call(&main_func, vec![])?;
            Ok(res.as_u64() as i32)
        } else {
            Ok(0) // Return 0 if no main function
        }
    }

    pub fn eval_statement(&mut self, stmt: &Statement) -> Result<(), EvalError> {
        match stmt {
            Statement::VariableDeclaration { ty, name, initializer } => {
                let val = if let Some(expr) = initializer {
                    let right_val = self.eval_expression(expr)?;
                    if !matches!(expr, Expression::HexLiteral(_, _)) {
                        check_implicit_conversion_warning(right_val.get_type(), *ty);
                    }
                    Value::from_u64(right_val.as_u64(), *ty)
                } else {
                    Value::default_val(*ty)
                };
                let last = self.frames.last_mut().unwrap();
                last.scopes.last_mut().unwrap().insert(name.clone(), VariableState {
                    val,
                    is_alive: true,
                    ty: *ty,
                });
            }
            Statement::ArrayDeclaration { ty, size, name, initializer } => {
                let size_val = match size {
                    ArraySize::Literal(v) => *v,
                    ArraySize::Variable(var_name) => {
                        self.eval_expression(&Expression::Variable(var_name.clone()))?.as_u64()
                    }
                };
                let mut elements = Vec::new();
                if let Some(init_exprs) = initializer {
                    for expr in init_exprs {
                        let right_val = self.eval_expression(expr)?;
                        if !matches!(expr, Expression::HexLiteral(_, _)) {
                            check_implicit_conversion_warning(right_val.get_type(), *ty);
                        }
                        elements.push(Some(Value::from_u64(right_val.as_u64(), *ty)));
                    }
                } else {
                    for _ in 0..size_val {
                        elements.push(Some(Value::default_val(*ty)));
                    }
                }
                let last = self.frames.last_mut().unwrap();
                last.scopes.last_mut().unwrap().insert(name.clone(), VariableState {
                    val: Value::Array(*ty, elements),
                    is_alive: true,
                    ty: *ty,
                });
            }
            Statement::FunctionDeclaration { name, .. } => {
                self.functions.insert(name.clone(), stmt.clone());
            }
            Statement::Expression(expr) => {
                self.eval_expression(expr)?;
            }
            Statement::Loop(body) => {
                loop {
                    let initial_len = self.frames.last().unwrap().scopes.len();
                    self.frames.last_mut().unwrap().scopes.push(HashMap::new());
                    let res = (|| -> Result<(), EvalError> {
                        for s in body {
                            self.eval_statement(s)?;
                        }
                        Ok(())
                    })();
                    self.frames.last_mut().unwrap().scopes.truncate(initial_len);
                    res?;
                }
            }
            Statement::TryCatch { try_body, catch_clauses } => {
                let initial_len = self.frames.last().unwrap().scopes.len();
                self.frames.last_mut().unwrap().scopes.push(HashMap::new());
                let res = (|| -> Result<(), EvalError> {
                    for s in try_body {
                        self.eval_statement(s)?;
                    }
                    Ok(())
                })();
                self.frames.last_mut().unwrap().scopes.truncate(initial_len);

                match res {
                    Err(EvalError::Exception(code)) => {
                        let mut handled = false;
                        for clause in catch_clauses {
                            if clause.exception_code == code {
                                let catch_initial_len = self.frames.last().unwrap().scopes.len();
                                self.frames.last_mut().unwrap().scopes.push(HashMap::new());
                                let catch_res = (|| -> Result<(), EvalError> {
                                    for s in &clause.body {
                                        self.eval_statement(s)?;
                                    }
                                    Ok(())
                                })();
                                self.frames.last_mut().unwrap().scopes.truncate(catch_initial_len);
                                catch_res?;
                                handled = true;
                                break;
                            }
                        }
                        if !handled {
                            return Err(EvalError::Exception(code));
                        }
                    }
                    Err(e) => return Err(e),
                    Ok(_) => {}
                }
            }
            Statement::Throw { exception_code, condition } => {
                let cond_val = self.eval_expression(condition)?.as_u64();
                if cond_val != 0 {
                    return Err(EvalError::Exception(*exception_code));
                }
            }
        }
        Ok(())
    }

    fn eval_expression(&mut self, expr: &Expression) -> Result<Value, EvalError> {
        match expr {
            Expression::HexLiteral(val, is_neg) => {
                let mut v = *val as i64;
                if *is_neg {
                    v = -v;
                }
                // default literal is parsed as something, but we need context for type.
                // In fuga, types are inferred by assignment, but literal itself is just a number.
                // We'll return it as I64 and cast when assigned.
                Ok(Value::I64(v))
            }
            Expression::Variable(name) => {
                let (val, is_local, frame_idx) = self.lookup_variable(name)?;
                if is_local {
                    // Consume the variable in local frame
                    let frame = &mut self.frames[frame_idx];
                    for scope in frame.scopes.iter_mut().rev() {
                        if let Some(var_state) = scope.get_mut(name) {
                            var_state.is_alive = false;
                            break;
                        }
                    }
                }
                Ok(val.clone())
            }
            Expression::ArrayElement(name, index_expr) => {
                let index = self.eval_expression(index_expr)?.as_u64() as usize;
                let (mut arr_val, is_local, frame_idx) = self.lookup_variable(name)?;
                
                if let Value::Array(ty, ref mut elements) = arr_val {
                    if let Some(elem) = elements.get_mut(index) {
                        if let Some(v) = elem {
                            let ret = v.clone();
                            if is_local {
                                // Consume element
                                *elem = None;
                                let frame = &mut self.frames[frame_idx];
                                for scope in frame.scopes.iter_mut().rev() {
                                    if let Some(var_state) = scope.get_mut(name) {
                                        var_state.val = Value::Array(ty, elements.clone());
                                        break;
                                    }
                                }
                            }
                            Ok(ret)
                        } else {
                            Err(EvalError::ConsumedVariable(format!("{}[{}]", name, index)))
                        }
                    } else {
                        Err(EvalError::UndefinedVariable(format!("{}[{}]", name, index)))
                    }
                } else {
                    Err(EvalError::TypeMismatch)
                }
            }
            Expression::UnaryOp(op, inner) => {
                if *op == UnaryOperator::Exists {
                    // ? operator does not consume
                    if let Expression::Variable(name) = &**inner {
                        if let Ok((_, is_local, frame_idx)) = self.lookup_variable(name) {
                            let frame = &self.frames[frame_idx];
                            let mut alive = false;
                            if is_local {
                                for scope in frame.scopes.iter().rev() {
                                    if let Some(var) = scope.get(name) {
                                        alive = var.is_alive;
                                        break;
                                    }
                                }
                            } else {
                                for scope in frame.scopes.iter().rev() {
                                    if let Some(var) = scope.get(name) {
                                        alive = var.is_alive;
                                        break;
                                    }
                                }
                            }
                            return Ok(Value::I64(if alive { 1 } else { 0 }));
                        } else {
                            return Ok(Value::I64(0));
                        }
                    }
                }
                let val = self.eval_expression(inner)?;
                match op {
                    UnaryOperator::BitwiseNot => Ok(Value::I64(!val.as_u64() as i64)),
                    _ => Ok(Value::I64(0)),
                }
            }
            Expression::BinaryOp(op, lhs, rhs) => {
                if *op == BinaryOperator::Assign {
                    return self.eval_assignment(lhs, rhs);
                }

                let left = self.eval_expression(lhs)?;
                let right = self.eval_expression(rhs)?;

                let res = match op {
                    BinaryOperator::Add => left.as_u64().wrapping_add(right.as_u64()),
                    BinaryOperator::Sub => left.as_u64().wrapping_sub(right.as_u64()),
                    BinaryOperator::BitwiseAnd => left.as_u64() & right.as_u64(),
                    BinaryOperator::BitwiseOr => left.as_u64() | right.as_u64(),
                    BinaryOperator::ShiftLeft => left.as_u64() << right.as_u64(),
                    BinaryOperator::ShiftRight => {
                        if Value::is_signed(left.get_type()) {
                            (left.as_i64() >> right.as_u64()) as u64
                        } else {
                            left.as_u64() >> right.as_u64()
                        }
                    },
                    BinaryOperator::Equal => if left.as_u64() == right.as_u64() { 1 } else { 0 },
                    BinaryOperator::LessThan => {
                        if Value::is_signed(left.get_type()) {
                            if left.as_i64() < right.as_i64() { 1 } else { 0 }
                        } else {
                            if left.as_u64() < right.as_u64() { 1 } else { 0 }
                        }
                    },
                    BinaryOperator::GreaterThan => {
                        if Value::is_signed(left.get_type()) {
                            if left.as_i64() > right.as_i64() { 1 } else { 0 }
                        } else {
                            if left.as_u64() > right.as_u64() { 1 } else { 0 }
                        }
                    },
                    _ => 0,
                };
                Ok(Value::I64(res as i64))
            }
            Expression::FunctionCall(name, args) => {
                if name == "_putc" {
                    let arg = self.eval_expression(&args[0])?;
                    let mut stdout = std::io::stdout();
                    let buf = [arg.as_u64() as u8];
                    if stdout.write_all(&buf).is_err() {
                        return Err(EvalError::Exception(0xFFFF0000));
                    }
                    let _ = stdout.flush();
                    return Ok(Value::I32(0));
                } else if name == "_getc" {
                    let mut stdin = std::io::stdin();
                    let mut buf = [0; 1];
                    if stdin.read_exact(&mut buf).is_err() {
                        return Err(EvalError::Exception(0xFFFF0001));
                    }
                    return Ok(Value::U8(buf[0]));
                }
                
                let func_stmt = self.functions.get(name).cloned().ok_or(EvalError::UndefinedVariable(name.clone()))?;
                self.eval_function_call(&func_stmt, args.clone())
            }
        }
    }

    fn eval_assignment(&mut self, lhs: &Expression, rhs: &Expression) -> Result<Value, EvalError> {
        // Handle self-assignment
        let right_val = self.eval_expression(rhs)?;

        match lhs {
            Expression::Variable(name) => {
                let last_idx = self.frames.len() - 1;
                let frame = &mut self.frames[last_idx];
                
                let mut found = false;
                for scope in frame.scopes.iter_mut().rev() {
                    if let Some(var_state) = scope.get_mut(name) {
                        if !matches!(rhs, Expression::HexLiteral(_, _)) {
                            check_implicit_conversion_warning(right_val.get_type(), var_state.ty);
                        }
                        var_state.val = Value::from_u64(right_val.as_u64(), var_state.ty);
                        var_state.is_alive = true; // Revive it
                        found = true;
                        break;
                    }
                }
                
                if !found {
                    // Check outer scopes to throw read-only error
                    for i in (0..last_idx).rev() {
                        for scope in self.frames[i].scopes.iter().rev() {
                            if scope.contains_key(name) {
                                return Err(EvalError::UndefinedVariable(format!("{} is read-only", name)));
                            }
                        }
                    }
                    return Err(EvalError::UndefinedVariable(name.clone()));
                }
            }
            Expression::ArrayElement(name, index_expr) => {
                let index = self.eval_expression(index_expr)?.as_u64() as usize;
                let last_idx = self.frames.len() - 1;
                let frame = &mut self.frames[last_idx];
                
                let mut found = false;
                for scope in frame.scopes.iter_mut().rev() {
                    if let Some(var_state) = scope.get_mut(name) {
                        found = true;
                        if let Value::Array(ty, ref mut elements) = var_state.val {
                            if let Some(elem) = elements.get_mut(index) {
                                if elem.is_some() {
                                    if !matches!(rhs, Expression::HexLiteral(_, _)) {
                                        check_implicit_conversion_warning(right_val.get_type(), ty);
                                    }
                                    *elem = Some(Value::from_u64(right_val.as_u64(), ty));
                                } else {
                                    return Err(EvalError::ConsumedVariable(format!("{}[{}]", name, index)));
                                }
                            } else {
                                return Err(EvalError::UndefinedVariable(format!("{}[{}]", name, index)));
                            }
                        }
                        break;
                    }
                }

                if !found {
                    for i in (0..last_idx).rev() {
                        for scope in self.frames[i].scopes.iter().rev() {
                            if scope.contains_key(name) {
                                return Err(EvalError::UndefinedVariable(format!("{} is read-only", name)));
                            }
                        }
                    }
                    return Err(EvalError::UndefinedVariable(format!("{}[{}]", name, index)));
                }
            }
            _ => return Err(EvalError::TypeMismatch),
        }

        Ok(right_val)
    }

    fn eval_function_call(&mut self, func_stmt: &Statement, args: Vec<Expression>) -> Result<Value, EvalError> {
        if let Statement::FunctionDeclaration { ret_type, name, params, body } = func_stmt {
            let mut evaled_args = Vec::new();
            for arg in &args {
                evaled_args.push(self.eval_expression(arg)?);
            }

            let mut new_frame = Frame { scopes: vec![HashMap::new()] };
            for (i, (ty, p_name)) in params.iter().enumerate() {
                if !matches!(args[i], Expression::HexLiteral(_, _)) {
                    check_implicit_conversion_warning(evaled_args[i].get_type(), *ty);
                }
                let val = Value::from_u64(evaled_args[i].as_u64(), *ty);
                new_frame.scopes[0].insert(p_name.clone(), VariableState { val, is_alive: true, ty: *ty });
            }

            // Implicit return variable
            let ret_var_name = name.clone();
            new_frame.scopes[0].insert(ret_var_name.clone(), VariableState {
                val: Value::default_val(*ret_type),
                is_alive: true,
                ty: *ret_type,
            });

            self.frames.push(new_frame);

            let res = (|| -> Result<(), EvalError> {
                for s in body {
                    self.eval_statement(s)?;
                }
                Ok(())
            })();

            match res {
                Err(e) => {
                    self.frames.pop();
                    return Err(e);
                }
                Ok(_) => {
                    let mut frame = self.frames.pop().unwrap();
                    // Return variable should be in the root scope (scopes[0])
                    let ret_val = frame.scopes[0].get(&ret_var_name).unwrap().val.clone();
                    Ok(ret_val)
                }
            }
        } else {
            Err(EvalError::UndefinedVariable("Not a function".to_string()))
        }
    }

    fn lookup_variable(&self, name: &str) -> Result<(Value, bool, usize), EvalError> {
        let last_idx = self.frames.len() - 1;
        
        // 1. Check local scope
        let frame = &self.frames[last_idx];
        for scope in frame.scopes.iter().rev() {
            if let Some(var) = scope.get(name) {
                if var.is_alive {
                    return Ok((var.val.clone(), true, last_idx));
                } else {
                    return Err(EvalError::ConsumedVariable(name.to_string()));
                }
            }
        }

        // 2. Check outer scopes (read-only)
        for i in (0..last_idx).rev() {
            let frame = &self.frames[i];
            for scope in frame.scopes.iter().rev() {
                if let Some(var) = scope.get(name) {
                    if var.is_alive {
                        return Ok((var.val.clone(), false, i));
                    } else {
                        return Err(EvalError::ConsumedVariable(name.to_string()));
                    }
                }
            }
        }

        Err(EvalError::UndefinedVariable(name.to_string()))
    }
}
