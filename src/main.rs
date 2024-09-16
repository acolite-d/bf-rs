mod brainfuck;

use brainfuck::{interpreter::Interpreter, ir::IR, jit::Jit, program::Program, Eval};
use std::{env, ffi::c_void, fs, process};

fn main() {
    // Keep it simple for now, just load the first argument as file,
    // read it as a Brainfuck program, interpret it.

    if let Some(path) = env::args().nth(1) {
        if let Ok(source_code) = fs::read_to_string(&path) {
            // Interpreter::eval_source(Program::new(&source_code)).unwrap();

            let folded_ir: IR = Program::new(&source_code).into();
            folded_ir.backpatch_jumps();
            dbg!(&folded_ir);

            let jitted_program = Jit::eval_ir(folded_ir).unwrap();

            let arr = [0u8; 30_000];

            jitted_program(arr.as_ptr());

            dbg!(&arr[0..10]);
        } else {
            eprintln!("Failed to open file {}", path);
            process::exit(-1)
        }
    } else {
        println!("No args provided.");
        process::exit(-1);
    }
}
