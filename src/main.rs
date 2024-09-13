mod brainfuck;

use brainfuck::{interpreter::Interpreter, ir::IR, program::Program, Eval};
use std::{env, fs, num::NonZero, process, slice};

use nix::sys::mman::{mmap_anonymous, MapFlags, ProtFlags};
type MyJittedFn = unsafe extern "C" fn(a: u64, b: u64) -> u64;

use std::io::Write;

fn compile_my_function() -> MyJittedFn {
    let mut exec_mem: &mut [u8] = unsafe {
        let ptr = mmap_anonymous(
            None,
            NonZero::new_unchecked(4096),
            ProtFlags::PROT_READ | ProtFlags::PROT_WRITE | ProtFlags::PROT_EXEC,
            MapFlags::MAP_PRIVATE | MapFlags::MAP_ANONYMOUS,
        )
        .expect("Failed to get executable memory from OS for JIT compilation!");

        slice::from_raw_parts_mut(ptr.as_ptr().cast::<u8>(), 4096)
    };

    // x86_64 calling convention. First pass to rdi, rsi, rdx, rcx, r8, r9, <stack>
    // rax is the return register
    //
    // addq is the instruction we can use to add two 64 bit unsigned numbers,
    // addq %

    exec_mem[0..4].copy_from_slice(&[0x48, 0x89, 0xf0, 0xc3]);

    dbg!(&exec_mem[0..4]);

    unsafe { std::mem::transmute(exec_mem.as_ptr()) }
}

fn main() {
    // Keep it simple for now, just load the first argument as file,
    // read it as a Brainfuck program, interpret it.

    if let Some(path) = env::args().nth(1) {
        if let Ok(source_code) = fs::read_to_string(&path) {
            let folded_ir: IR = Program::new(&source_code).into();
            dbg!(&folded_ir);
        } else {
            eprintln!("Failed to open file {}", path);
            process::exit(-1)
        }
    } else {
        println!("No args provided.");
        process::exit(-1);
    }
}
