pub mod analyzer;
pub mod codegen;

use crate::ast::Program;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CompileError {
    #[error("Analyzer error: {0}")]
    AnalyzerError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("C compiler error: {0}")]
    GccError(String),
}

pub fn compile(
    program: &Program,
    source_file: &Path,
    output: Option<&Path>,
    emit_c: bool,
) -> Result<(), CompileError> {
    // 1. Semantic Analysis (Liveness etc.)
    analyzer::analyze(program).map_err(CompileError::AnalyzerError)?;

    // 2. Code Generation
    let c_code = codegen::generate_c(program);

    // 3. Determine output filenames
    let default_output = source_file.with_extension("");
    let bin_path = output.unwrap_or(&default_output);
    let c_path = bin_path.with_extension("c");

    // Write C code to file
    fs::write(&c_path, &c_code)?;

    // Compile with GCC
    let status = Command::new("gcc")
        .arg(&c_path)
        .arg("-o")
        .arg(bin_path)
        .arg("-O2") // Some basic optimizations
        .status()?;

    if !status.success() {
        return Err(CompileError::GccError("gcc compilation failed".to_string()));
    }

    // Clean up C file if emit_c is not requested
    if !emit_c {
        let _ = fs::remove_file(&c_path);
    }

    Ok(())
}
