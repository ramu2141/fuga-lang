mod ast;
mod lexer;
mod parser;
mod interpreter;
pub mod compiler;

use clap::Parser;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "fuga")]
#[command(about = "fuga language compiler and interpreter")]
struct Cli {
    /// The source file to execute or compile
    source_file: PathBuf,

    /// Compile mode
    #[arg(short, long)]
    compile: bool,

    /// Output path (for compile mode)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Emit C source code instead of or in addition to binary
    #[arg(long)]
    emit_c: bool,
}

fn main() {
    let cli = Cli::parse();

    let src_code = match fs::read_to_string(&cli.source_file) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Failed to read source file: {}", e);
            std::process::exit(1);
        }
    };

    if cli.compile {
        let mut lexer = lexer::Lexer::new(&src_code);
        let tokens = match lexer.tokenize() {
            Ok(tokens) => tokens,
            Err(e) => {
                eprintln!("Lexer error: {}", e);
                std::process::exit(1);
            }
        };

        let mut parser = parser::Parser::new(tokens);
        let program = match parser.parse_program() {
            Ok(prog) => prog,
            Err(e) => {
                eprintln!("Parse error: {}", e);
                std::process::exit(1);
            }
        };

        if let Err(e) = compiler::compile(&program, &cli.source_file, cli.output.as_deref(), cli.emit_c) {
            eprintln!("Compiler error: {}", e);
            std::process::exit(1);
        }
    } else {
        // Interpreter mode
        let mut lexer = lexer::Lexer::new(&src_code);
        let tokens = match lexer.tokenize() {
            Ok(tokens) => tokens,
            Err(e) => {
                eprintln!("Lexer error: {}", e);
                std::process::exit(1);
            }
        };

        let mut parser = parser::Parser::new(tokens);
        let program = match parser.parse_program() {
            Ok(prog) => prog,
            Err(e) => {
                eprintln!("Parse error: {}", e);
                std::process::exit(1);
            }
        };

        if let Err(e) = compiler::analyzer::analyze(&program) {
            eprintln!("Analyzer error: {}", e);
            std::process::exit(1);
        }

        let mut interpreter = interpreter::Interpreter::new();
        match interpreter.run_program(&program) {
            Ok(exit_code) => {
                std::process::exit(exit_code);
            }
            Err(e) => {
                eprintln!("Runtime error: {:?}", e);
                std::process::exit(1);
            }
        }
    }
}
