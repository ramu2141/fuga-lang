use crate::ast::*;
use std::collections::HashSet;

pub fn analyze(program: &Program) -> Result<(), String> {
    for stmt in program {
        if let Statement::FunctionDeclaration { name, params, body, .. } = stmt {
            analyze_function(name, params, body)?;
        }
    }
    Ok(())
}

fn analyze_function(
    func_name: &str,
    params: &[(Type, String)],
    body: &[Statement],
) -> Result<(), String> {
    // Local liveness tracking
    let mut declared = vec![HashSet::new()];
    let mut consumed = HashSet::new();

    // Parameters are declared and alive
    for (_, param_name) in params {
        declared[0].insert(param_name.clone());
    }

    // Function return variable is implicitly declared
    declared[0].insert(func_name.to_string());

    analyze_statements(body, &mut declared, &mut consumed)?;

    Ok(())
}

fn is_declared(name: &str, declared: &Vec<HashSet<String>>) -> bool {
    for scope in declared.iter().rev() {
        if scope.contains(name) {
            return true;
        }
    }
    false
}

fn analyze_statements(
    stmts: &[Statement],
    declared: &mut Vec<HashSet<String>>,
    consumed: &mut HashSet<String>,
) -> Result<(), String> {
    for stmt in stmts {
        match stmt {
            Statement::VariableDeclaration { name, initializer, .. } => {
                if let Some(init) = initializer {
                    analyze_expression(init, declared, consumed)?;
                }
                declared.last_mut().unwrap().insert(name.clone());
                consumed.remove(name); // It is now alive
            }
            Statement::ArrayDeclaration { name, initializer, size, .. } => {
                match size {
                    ArraySize::Variable(var_name) => {
                        analyze_variable_read(var_name, declared, consumed)?;
                    }
                    ArraySize::Literal(_) => {}
                }
                if let Some(init_list) = initializer {
                    for init in init_list {
                        analyze_expression(init, declared, consumed)?;
                    }
                }
                declared.last_mut().unwrap().insert(name.clone());
                consumed.remove(name);
            }
            Statement::Expression(expr) => {
                analyze_expression(expr, declared, consumed)?;
            }
            Statement::Loop(loop_body) => {
                // For a loop, we do a basic analysis. 
                declared.push(HashSet::new());
                analyze_statements(loop_body, declared, consumed)?;
                declared.pop();
            }
            Statement::TryCatch { try_body, catch_clauses } => {
                let mut try_consumed = consumed.clone();
                declared.push(HashSet::new());
                analyze_statements(try_body, declared, &mut try_consumed)?;
                declared.pop();
                
                for catch in catch_clauses {
                    let mut catch_consumed = consumed.clone();
                    declared.push(HashSet::new());
                    analyze_statements(&catch.body, declared, &mut catch_consumed)?;
                    declared.pop();
                    try_consumed.extend(catch_consumed);
                }
                *consumed = try_consumed;
            }
            Statement::Throw { condition, .. } => {
                analyze_expression(condition, declared, consumed)?;
            }
            Statement::FunctionDeclaration { name, .. } => {
                return Err(format!("Syntax Error: Function declaration '{}' is prohibited inside a function.", name));
            }
        }
    }
    Ok(())
}

fn analyze_expression(
    expr: &Expression,
    declared: &mut Vec<HashSet<String>>,
    consumed: &mut HashSet<String>,
) -> Result<(), String> {
    match expr {
        Expression::HexLiteral(_, _) => Ok(()),
        Expression::Variable(name) => {
            analyze_variable_read(name, declared, consumed)
        }
        Expression::ArrayElement(name, index_expr) => {
            analyze_expression(index_expr, declared, consumed)?;
            // Array elements are consumed individually in runtime, but statically it's hard.
            // We just ensure the array name is declared.
            if is_declared(name, declared) && consumed.contains(name) {
                return Err(format!("Cannot access array '{}' because it might be fully consumed.", name));
            }
            Ok(())
        }
        Expression::BinaryOp(op, left, right) => {
            if *op == BinaryOperator::Assign {
                // RHS is evaluated first
                analyze_expression(right, declared, consumed)?;
                
                // LHS
                if let Expression::Variable(name) = &**left {
                    // Outer scope assignment is prohibited
                    if !is_declared(name, declared) {
                         return Err(format!("Cannot assign to dynamically scoped variable '{}' (Read-Only)", name));
                    }
                    // It's assigned, so it becomes alive again
                    consumed.remove(name);
                } else if let Expression::ArrayElement(name, index_expr) = &**left {
                    analyze_expression(index_expr, declared, consumed)?;
                    if !is_declared(name, declared) {
                        return Err(format!("Cannot assign to dynamically scoped array '{}' (Read-Only)", name));
                    }
                }
                Ok(())
            } else {
                analyze_expression(left, declared, consumed)?;
                analyze_expression(right, declared, consumed)?;
                Ok(())
            }
        }
        Expression::UnaryOp(_, inner) => {
            analyze_expression(inner, declared, consumed)
        }
        Expression::FunctionCall(_, args) => {
            for arg in args {
                analyze_expression(arg, declared, consumed)?;
            }
            Ok(())
        }
    }
}

fn analyze_variable_read(
    name: &str,
    declared: &mut Vec<HashSet<String>>,
    consumed: &mut HashSet<String>,
) -> Result<(), String> {
    if is_declared(name, declared) {
        if consumed.contains(name) {
            return Err(format!("Variable '{}' is used after being consumed.", name));
        }
        // Consume it
        consumed.insert(name.to_string());
    } else {
        // If not locally declared, we assume it's dynamically scoped from an outer frame.
        // Dynamically scoped variables are readonly and not consumed.
    }
    Ok(())
}
