mod brainfuck;

use brainfuck::{interpreter::Interpreter, ir::IR, program::Program, Eval};
use std::{env, fs, process};

fn main() {
    // Keep it simple for now, just load the first argument as file,
    // read it as a Brainfuck program, interpret it.
    if let Some(path) = env::args().nth(1) {
        if let Ok(source_code) = fs::read_to_string(&path) {
            let program = Program::new(&source_code);
            dbg!(&program);
            let ir: IR = program.into();
            dbg!(&ir);
        } else {
            eprintln!("Failed to open file {}", path);
            process::exit(-1)
        }
    } else {
        println!("No args provided.");
        process::exit(-1);
    }
}
