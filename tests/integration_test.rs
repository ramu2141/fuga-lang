use std::process::Command;
use std::env;

#[test]
fn test_hello_world() {
    let mut dir = env::current_dir().unwrap();
    dir.push("target/debug/f-lang");
    
    let output = Command::new(dir)
        .arg("tests/scripts/hello.fuga")
        .output()
        .expect("Failed to execute f-lang");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "Hello, World\n");
}

fn run_fuga_script(script_name: &str, expect_success: bool) {
    let mut dir = env::current_dir().unwrap();
    dir.push("target/debug/f-lang");
    
    // 1. Test Interpreter
    let output_interp = Command::new(&dir)
        .arg(format!("tests/scripts/{}", script_name))
        .output()
        .expect("Failed to execute f-lang interpreter");

    if expect_success {
        if !output_interp.status.success() {
            println!("Interpreter Stdout: {}", String::from_utf8_lossy(&output_interp.stdout));
            println!("Interpreter Stderr: {}", String::from_utf8_lossy(&output_interp.stderr));
            panic!("Expected success but failed in interpreter: {}", script_name);
        }
    } else {
        if output_interp.status.success() {
            panic!("Expected failure but succeeded in interpreter: {}", script_name);
        }
    }

    // 2. Test Compiler
    let bin_path = format!("tests/scripts/{}.bin", script_name);
    let output_comp = Command::new(&dir)
        .arg("--compile")
        .arg(format!("tests/scripts/{}", script_name))
        .arg("-o")
        .arg(&bin_path)
        .output()
        .expect("Failed to execute f-lang compiler");

    if expect_success {
        if !output_comp.status.success() {
            println!("Compiler Stdout: {}", String::from_utf8_lossy(&output_comp.stdout));
            println!("Compiler Stderr: {}", String::from_utf8_lossy(&output_comp.stderr));
            panic!("Compiler failed to compile script: {}", script_name);
        }

        // Run the generated binary
        let output_bin = Command::new(&bin_path)
            .output()
            .expect("Failed to execute generated binary");

        if !output_bin.status.success() {
            println!("Binary Stdout: {}", String::from_utf8_lossy(&output_bin.stdout));
            println!("Binary Stderr: {}", String::from_utf8_lossy(&output_bin.stderr));
            panic!("Generated binary failed: {}", script_name);
        }
        
        let _ = std::fs::remove_file(&bin_path);
    } else {
        // If it's expected to fail, it could fail during compilation (static analysis) 
        // or during runtime (generated binary throws unhandled).
        if output_comp.status.success() {
            let output_bin = Command::new(&bin_path)
                .output()
                .expect("Failed to execute generated binary");
            if output_bin.status.success() {
                panic!("Expected failure but succeeded in compiled binary: {}", script_name);
            }
            let _ = std::fs::remove_file(&bin_path);
        }
    }
}

#[test] fn test_arithmetic() { run_fuga_script("arithmetic.fuga", true); }
#[test] fn test_self_assign() { run_fuga_script("self_assign.fuga", true); }
#[test] fn test_dynamic_scope() { run_fuga_script("dynamic_scope.fuga", true); }
#[test] fn test_try_catch() { run_fuga_script("try_catch.fuga", true); }
#[test] fn test_array() { run_fuga_script("array.fuga", true); }
#[test] fn test_consume_err() { run_fuga_script("consume_err.fuga", false); }
#[test] fn test_readonly_err() { run_fuga_script("readonly_err.fuga", false); }
#[test] fn test_loop() { run_fuga_script("loop.fuga", true); }
#[test] fn test_logical() { run_fuga_script("logical.fuga", true); }
#[test] fn test_fibonacci() { run_fuga_script("fibonacci.fuga", true); }
#[test] fn test_operations() { run_fuga_script("operations.fuga", true); }
#[test] fn test_types() { run_fuga_script("types.fuga", true); }
#[test] fn test_default_init() { run_fuga_script("default_init.fuga", true); }
#[test] fn test_array_consume() { run_fuga_script("array_consume.fuga", true); }
#[test] fn test_array_consume_err() { run_fuga_script("array_consume_err.fuga", false); }
#[test] fn test_block_scope() { run_fuga_script("block_scope.fuga", false); }
