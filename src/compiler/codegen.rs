use crate::ast::*;
use std::collections::{HashMap, HashSet};

pub fn generate_c(program: &Program) -> String {
    let mut code = String::new();
    let mut global_types = HashMap::new();

    // Collect all variable types to support correct casting of dynamic variables
    for stmt in program {
        collect_types(stmt, &mut global_types);
    }

    // 1. C Standard Includes and Runtime
    code.push_str(
r#"#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <setjmp.h>

// --- Fuga Runtime ---
typedef struct ExceptionEnv {
    jmp_buf buf;
    struct ExceptionEnv* next;
} ExceptionEnv;

ExceptionEnv* current_exception_env = NULL;
uint64_t current_exception_code = 0;

void fuga_throw(uint64_t code) {
    if (current_exception_env) {
        current_exception_code = code;
        longjmp(current_exception_env->buf, 1);
    } else {
        fprintf(stderr, "Unhandled exception: %lx\n", (unsigned long)code);
        exit(1);
    }
}

typedef struct ScopeNode {
    const char* name;
    void* ptr;
    struct ScopeNode* next;
} ScopeNode;

ScopeNode* current_scope = NULL;

void* get_dynamic_var(const char* name) {
    for (ScopeNode* node = current_scope; node != NULL; node = node->next) {
        if (strcmp(node->name, name) == 0) return node->ptr;
    }
    fprintf(stderr, "Undefined dynamic variable: %s\n", name);
    exit(1);
}

int32_t f__putc(uint64_t c) {
    putchar((int)c);
    fflush(stdout);
    return 0;
}

uint8_t f__getc() {
    int c = getchar();
    if (c == EOF) fuga_throw(0xFFFF0001);
    return (uint8_t)c;
}

"#);

    // 2. Forward Declarations
    for stmt in program {
        if let Statement::FunctionDeclaration { ret_type, name, params, .. } = stmt {
            let c_ret_type = type_to_c(*ret_type);
            code.push_str(&format!("{} f_{}(", c_ret_type, name));
            if params.is_empty() {
                code.push_str("void");
            } else {
                let params_c: Vec<String> = params.iter().map(|(ty, n)| format!("{} v_{}", type_to_c(*ty), n.trim_start_matches('$'))).collect();
                code.push_str(&params_c.join(", "));
            }
            code.push_str(");\n");
        }
    }
    code.push_str("\n");

    // 3. Function Implementations
    for stmt in program {
        if let Statement::FunctionDeclaration { ret_type, name, params, body } = stmt {
            let c_ret_type = type_to_c(*ret_type);
            code.push_str(&format!("{} f_{}(", c_ret_type, name));
            if params.is_empty() {
                code.push_str("void");
            } else {
                let params_c: Vec<String> = params.iter().map(|(ty, n)| format!("{} v_{}", type_to_c(*ty), n)).collect();
                code.push_str(&params_c.join(", "));
            }
            code.push_str(") {\n");
            
            // Save previous scope
            code.push_str("    ScopeNode* prev_scope = current_scope;\n");
            
            // Push params to scope
            for (_, n) in params {
                code.push_str(&format!("    ScopeNode sn_{0} = {{\"{0}\", &v_{0}, current_scope}};\n", n));
                code.push_str(&format!("    current_scope = &sn_{0};\n", n));
            }

            // Return variable
            let ret_var_name = name; // main -> v_main
            code.push_str(&format!("    {} v_{} = 0;\n", c_ret_type, ret_var_name));
            code.push_str(&format!("    ScopeNode sn_{0} = {{\"{0}\", &v_{0}, current_scope}};\n", ret_var_name));
            code.push_str(&format!("    current_scope = &sn_{0};\n", ret_var_name));

            // Generate body
            let mut local_vars = HashSet::new();
            for (_, n) in params {
                local_vars.insert(n.clone());
            }
            local_vars.insert(name.clone());

            generate_statements(body, &mut code, 1, &mut local_vars, &global_types);

            // Restore scope and return
            code.push_str("    current_scope = prev_scope;\n");
            code.push_str(&format!("    return v_{};\n", ret_var_name));
            code.push_str("}\n\n");
        }
    }

    // 4. C main function
    code.push_str("int main() {\n");
    
    let mut main_locals = HashSet::new();
    let mut top_level_stmts = Vec::new();
    for stmt in program {
        if !matches!(stmt, Statement::FunctionDeclaration{..}) {
            top_level_stmts.push(stmt.clone());
        }
    }
    
    generate_statements(&top_level_stmts, &mut code, 1, &mut main_locals, &global_types);
    
    code.push_str("    int32_t ret = f_main();\n");
    code.push_str("    return (int)ret;\n");
    code.push_str("}\n");

    code
}

fn type_to_c(ty: Type) -> &'static str {
    match ty {
        Type::I8 => "int8_t",
        Type::U8 => "uint8_t",
        Type::I16 => "int16_t",
        Type::U16 => "uint16_t",
        Type::I32 => "int32_t",
        Type::U32 => "uint32_t",
        Type::I64 => "int64_t",
        Type::U64 => "uint64_t",
    }
}

fn generate_statements(
    stmts: &[Statement], 
    code: &mut String, 
    indent: usize,
    local_vars: &mut HashSet<String>,
    global_types: &HashMap<String, Type>,
) {
    let ind = "    ".repeat(indent);
    for stmt in stmts {
        match stmt {
            Statement::VariableDeclaration { ty, name, initializer } => {
                let c_type = type_to_c(*ty);
                let v_name = name;
                local_vars.insert(name.clone());
                
                code.push_str(&format!("{}{} v_{} = ", ind, c_type, v_name));
                if let Some(init) = initializer {
                    code.push_str(&generate_expression(init, local_vars, global_types));
                } else {
                    code.push_str("0");
                }
                code.push_str(";\n");

                code.push_str(&format!("{ind}ScopeNode sn_{v_name} = {{\"{v_name}\", &v_{v_name}, current_scope}};\n"));
                code.push_str(&format!("{ind}current_scope = &sn_{v_name};\n"));
            }
            Statement::ArrayDeclaration { ty, size, name, initializer } => {
                let c_type = type_to_c(*ty);
                let v_name = name;
                local_vars.insert(name.clone());
                
                let size_str = match size {
                    ArraySize::Literal(v) => format!("{}", v),
                    ArraySize::Variable(n) => generate_expression(&Expression::Variable(n.clone()), local_vars, global_types),
                };

                // Allocate dynamically on C stack using VLA (Variable Length Array) - supported in C99
                code.push_str(&format!("{}{} v_{}[{}];\n", ind, c_type, v_name, size_str));
                code.push_str(&format!("{}uint8_t c_{}[{}];\n", ind, v_name, size_str));
                
                // Initialize if present
                if let Some(init_list) = initializer {
                    for (i, expr) in init_list.iter().enumerate() {
                        code.push_str(&format!("{}v_{}[{}] = {};\n", ind, v_name, i, generate_expression(expr, local_vars, global_types)));
                        code.push_str(&format!("{}c_{}[{}] = 1;\n", ind, v_name, i));
                    }
                } else {
                    code.push_str(&format!("{ind}memset(v_{v_name}, 0, sizeof({c_type}) * {size_str});\n"));
                    code.push_str(&format!("{ind}memset(c_{v_name}, 1, {size_str});\n"));
                }

                code.push_str(&format!("{ind}ScopeNode sn_{v_name} = {{\"{v_name}\", v_{v_name}, current_scope}};\n"));
                code.push_str(&format!("{ind}current_scope = &sn_{v_name};\n"));
            }
            Statement::Expression(expr) => {
                code.push_str(&format!("{}{};\n", ind, generate_expression(expr, local_vars, global_types)));
            }
            Statement::Loop(body) => {
                code.push_str(&format!("{}while(1) {{\n", ind));
                let inner_ind = format!("{}    ", ind);
                code.push_str(&format!("{}ScopeNode* loop_scope = current_scope;\n", inner_ind));
                
                let mut loop_vars = local_vars.clone();
                generate_statements(body, code, indent + 1, &mut loop_vars, global_types);
                
                code.push_str(&format!("{}current_scope = loop_scope;\n", inner_ind));
                code.push_str(&format!("{}}}\n", ind));
            }
            Statement::TryCatch { try_body, catch_clauses } => {
                code.push_str(&format!("{ind}{{\n"));
                let inner_ind = format!("{}    ", ind);
                code.push_str(&format!("{inner_ind}ScopeNode* try_scope = current_scope;\n"));
                
                code.push_str(&format!("{inner_ind}ExceptionEnv env_try;\n"));
                code.push_str(&format!("{inner_ind}env_try.next = current_exception_env;\n"));
                code.push_str(&format!("{inner_ind}current_exception_env = &env_try;\n"));
                
                code.push_str(&format!("{inner_ind}if (setjmp(env_try.buf) == 0) {{\n"));
                let mut try_vars = local_vars.clone();
                generate_statements(try_body, code, indent + 2, &mut try_vars, global_types);
                code.push_str(&format!("{inner_ind}    current_scope = try_scope;\n"));
                code.push_str(&format!("{inner_ind}}} else {{\n"));
                code.push_str(&format!("{inner_ind}    current_scope = try_scope;\n"));
                
                // Catch clauses
                code.push_str(&format!("{inner_ind}    uint64_t code = current_exception_code;\n"));
                for (i, catch) in catch_clauses.iter().enumerate() {
                    if i == 0 {
                        code.push_str(&format!("{inner_ind}    if (code == 0x{:X}) {{\n", catch.exception_code));
                    } else {
                        code.push_str(&format!("{inner_ind}    else if (code == 0x{:X}) {{\n", catch.exception_code));
                    }
                    let mut catch_vars = local_vars.clone();
                    generate_statements(&catch.body, code, indent + 3, &mut catch_vars, global_types);
                    code.push_str(&format!("{inner_ind}    }}\n"));
                }
                
                // If not caught, rethrow
                code.push_str(&format!("{inner_ind}    else {{\n"));
                code.push_str(&format!("{inner_ind}        current_exception_env = env_try.next;\n"));
                code.push_str(&format!("{inner_ind}        fuga_throw(code);\n"));
                code.push_str(&format!("{inner_ind}    }}\n"));
                
                code.push_str(&format!("{inner_ind}}}\n"));
                code.push_str(&format!("{inner_ind}current_exception_env = env_try.next;\n"));
                code.push_str(&format!("{ind}}}\n"));
            }
            Statement::Throw { exception_code, condition } => {
                let cond_str = generate_expression(condition, local_vars, global_types);
                code.push_str(&format!("{}if ({}) {{\n", ind, cond_str));
                code.push_str(&format!("{}    fuga_throw(0x{:X});\n", ind, exception_code));
                code.push_str(&format!("{}}}\n", ind));
            }
            _ => {}
        }
    }
}

fn generate_expression(expr: &Expression, local_vars: &HashSet<String>, global_types: &HashMap<String, Type>) -> String {
    match expr {
        Expression::HexLiteral(v, neg) => {
            if *neg {
                format!("-0x{:X}ULL", v)
            } else {
                format!("0x{:X}ULL", v)
            }
        }
        Expression::Variable(name) => {
            if local_vars.contains(name) {
                format!("v_{}", name)
            } else {
                // Dynamic scope read
                let ty = global_types.get(name).cloned().unwrap_or(Type::I32);
                let c_type = type_to_c(ty);
                format!("(*({}*)get_dynamic_var(\"{}\"))", c_type, name)
            }
        }
        Expression::ArrayElement(name, index_expr) => {
            let idx = generate_expression(index_expr, local_vars, global_types);
            if local_vars.contains(name) {
                format!("(c_{name}[{idx}] ? (c_{name}[{idx}] = 0, v_{name}[{idx}]) : (fprintf(stderr, \"Variable '%s' is used after being consumed.\\n\", \"{name}\"), exit(1), 0))", name=name, idx=idx)
            } else {
                let ty = global_types.get(name).cloned().unwrap_or(Type::I32);
                let c_type = type_to_c(ty);
                format!("((({}*)get_dynamic_var(\"{}\"))[{}])", c_type, name, idx)
            }
        }
        Expression::BinaryOp(op, left, right) => {
            if *op == BinaryOperator::Assign {
                if let Expression::Variable(name) = &**left {
                    if local_vars.contains(name) {
                        return format!("(v_{} = {})", name, generate_expression(right, local_vars, global_types));
                    } else {
                        // Dynamic assignment is read-only error
                        return format!("(fprintf(stderr, \"Cannot modify read-only variable '%s'\\n\", \"{}\"), exit(1), 0)", name);
                    }
                } else if let Expression::ArrayElement(name, idx_expr) = &**left {
                    if local_vars.contains(name) {
                        let idx = generate_expression(idx_expr, local_vars, global_types);
                        return format!("(c_{name}[{idx}] = 1, v_{name}[{idx}] = {val})", name=name, idx=idx, val=generate_expression(right, local_vars, global_types));
                    } else {
                        return format!("(fprintf(stderr, \"Cannot modify read-only variable '%s'\\n\", \"{}\"), exit(1), 0)", name);
                    }
                }
            }
            let l = generate_expression(left, local_vars, global_types);
            let r = generate_expression(right, local_vars, global_types);
            match op {
                BinaryOperator::Add => format!("({} + {})", l, r),
                BinaryOperator::Sub => format!("({} - {})", l, r),
                BinaryOperator::BitwiseAnd => format!("({} & {})", l, r),
                BinaryOperator::BitwiseOr => format!("({} | {})", l, r),
                BinaryOperator::ShiftLeft => format!("({} << {})", l, r),
                BinaryOperator::ShiftRight => format!("({} >> {})", l, r),
                BinaryOperator::LessThan => format!("({} < {})", l, r),
                BinaryOperator::GreaterThan => format!("({} > {})", l, r),
                BinaryOperator::Equal => format!("({} == {})", l, r),
                BinaryOperator::Assign => format!("({} = {})", l, r),
            }
        }
        Expression::UnaryOp(op, inner) => {
            let i = generate_expression(inner, local_vars, global_types);
            match op {
                UnaryOperator::BitwiseNot => format!("(~{})", i),
                UnaryOperator::Exists => format!("(1)"), // In compiler, we don't track dynamic consumption statically yet, so just 1
            }
        }
        Expression::FunctionCall(name, args) => {
            let args_c: Vec<String> = args.iter().map(|a| generate_expression(a, local_vars, global_types)).collect();
            format!("f_{}({})", name, args_c.join(", "))
        }
    }
}

pub fn collect_types(stmt: &Statement, global_types: &mut HashMap<String, Type>) {
    match stmt {
        Statement::VariableDeclaration { ty, name, .. } => {
            global_types.insert(name.clone(), *ty);
        }
        Statement::ArrayDeclaration { ty, name, .. } => {
            global_types.insert(name.clone(), *ty);
        }
        Statement::FunctionDeclaration { params, body, .. } => {
            for (ty, name) in params {
                global_types.insert(name.clone(), *ty);
            }
            for s in body {
                collect_types(s, global_types);
            }
        }
        Statement::Loop(body) => {
            for s in body {
                collect_types(s, global_types);
            }
        }
        Statement::TryCatch { try_body, catch_clauses } => {
            for s in try_body {
                collect_types(s, global_types);
            }
            for c in catch_clauses {
                for s in &c.body {
                    collect_types(s, global_types);
                }
            }
        }
        _ => {}
    }
}
