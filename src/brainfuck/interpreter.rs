use super::{
    ir::IR,
    program::{Operator, Program},
    Eval,
};
use std::ffi::c_int;

extern "C" {
    fn getchar() -> c_int;
    fn putchar(c: c_int) -> c_int;
}

pub struct Interpreter;

impl Eval for Interpreter {
    // The interpreter just executes the program, returns nothing
    type Output = ();

    fn eval_source(program: Program) -> Result<Self::Output, ()> {
        // According to this source, https://gist.github.com/roachhd/dce54bec8ba55fb17d3a
        // standard Brainfuck has 30,000 bytes of memory to work with,
        // so initialize an array of 30,000 bytes to start.
        let mut mem = [0u8; 30_000];

        // Work with two pointers, one for memory, or tape, the other as an instruction pointer
        // that points to the current brainfuck operator. Both of these are array offsets, technically
        // not pointers, but can be thought of as such.
        let mut mem_ptr = 0usize;
        let mut ip = 0usize;

        // Interpreter loop, go operator by operator according to the IP,
        // execute the right code in every match arm. Exit loop when we've
        // reached the last operator in our code
        while ip < program.code.len() {
            match program.code[ip] {
                Operator::IncrementPtr => mem_ptr += 1,

                Operator::DecrementPtr => mem_ptr -= 1,

                Operator::IncrementValue => mem[mem_ptr] = mem[mem_ptr].wrapping_add(1),

                Operator::DecrementValue => mem[mem_ptr] = mem[mem_ptr].wrapping_sub(1),

                Operator::JumpIfZero => {
                    if mem[mem_ptr] == 0 {
                        ip = program.fwd_jump_table[&ip]
                    }
                }

                Operator::JumpIfNonZero => {
                    if mem[mem_ptr] != 0 {
                        ip = program.bwd_jump_table[&ip];
                    }
                }

                Operator::GetChar => unsafe {
                    mem[mem_ptr] = getchar() as u8;
                },

                Operator::PutChar => unsafe {
                    putchar(mem[mem_ptr] as c_int);
                },
            }

            // Don't forget to increment the instruction pointer for next operation!
            ip += 1;
        }

        Ok(())
    }

    fn eval_ir(ir: IR) -> Result<Self::Output, ()> {
        unimplemented!()
    }
}
