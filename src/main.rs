mod brainfuck;
mod cli;

use anyhow::{Context, Result};
use brainfuck::{interpreter::Interpreter, ir::IR, jit::Jit, program::Program, Eval};
use clap::Parser;
use cli::{Cli, Mode};
use std::{env, ffi::c_void, fs, process};

fn main() -> Result<()> {
    let cli = Cli::parse();

    let source_code = fs::read_to_string(cli.file.as_path())
        .with_context(|| format!("Failed to read source code from {}", cli.file.display()))?;

    let program = Program::new(&source_code)?;

    match cli.mode {
        Mode::Interpret => {
            // Interpreter::eval_source(program).unwrap();
            let ir: IR = program.into();
            Interpreter::eval_ir(ir)?;
        }

        Mode::Jit => {
            let ir: IR = program.into();
            let compiled_fn = Jit::eval_ir(ir)?;
            compiled_fn.run();
        }
    }

    Ok(())
}
