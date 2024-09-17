mod brainfuck;
mod cli;

use brainfuck::{interpreter::Interpreter, ir::IR, jit::Jit, program::Program, Eval};
use clap::Parser;
use cli::{Cli, Mode};
use std::{env, ffi::c_void, fs, process};

fn main() {
    // Keep it simple for now, just load the first argument as file,
    // read it as a Brainfuck program, interpret it.

    let cli = Cli::parse();

    if let Some(ref filepath) = cli.file {
        if let Ok(source_code) = fs::read_to_string(filepath) {
            //Interpreter::eval_source(Program::new(&source_code)).unwrap();

            let program = Program::new(&source_code);

            match cli.mode {
                Mode::Interpret => {
                    Interpreter::eval_source(program).unwrap();
                }

                Mode::Jit => {
                    let ir: IR = program.into();
                    ir.backpatch_jumps();
                    Jit::eval_ir(ir).unwrap().run();
                }
            }
        } else {
            eprintln!("Failed to open file {}", filepath.display());
            process::exit(-1)
        }
    } else {
        println!("No args provided.");
        process::exit(-1);
    }
}
