use super::{
    ir::{IRInsn, IR},
    program::Program,
    Eval,
};
use anyhow::Result;
use windows::Wdk::System::SystemServices::KeInvalidateAllCaches;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Console;
use windows::Win32::System::Memory::{self, VirtualAlloc};

use std::{
    ffi::{c_int, c_ulong, c_void},
    io::Write,
    num::NonZero,
    ptr::NonNull,
    slice,
};

extern "C" {
    fn putchar(c: c_int) -> c_int;
    fn getchar() -> c_int;
}

pub struct Jit;

// The Jit produces JittedFunctions from Brainfuck IR, its a tuple struct
// with a void pointer, and a size of memory pointed to by pointer (weird slice)
pub struct JittedFunction(*mut c_void, usize);

impl JittedFunction {
    pub fn run(&self) {
        // Converting a pointer of bytes to a function pointer in Rust is, as one would expect,
        // very unsafe. This requires an intrinsics function changing arbitrary memory objects
        // called "transmute".
        let function =
            unsafe { std::mem::transmute::<*mut c_void, extern "C" fn(*mut u8)>(self.0) };

        let mut mem = [0u8; 30_000];

        // Call the function
        unsafe { function(mem.as_mut_ptr()) }
    }
}

// Keeping in touch with Rust's stance on RAII driven design, implemented
// Drop for the JittedFunction object, which call syscall munmap(2) to
// relinquish the executable region of memory we requested from Linux
impl Drop for JittedFunction {
    fn drop(&mut self) {
        unsafe {
            Memory::VirtualFree(self.0, self.1, Memory::MEM_DECOMMIT)
                .expect("Should be able to release memory back to OS");
        }
    }
}

struct JumpPairPos {
    fwd_jmp: usize,
    bwd_jmp: usize,
}

impl Eval for Jit {
    type Output = JittedFunction;

    fn eval_source(src: Program) -> Result<Self::Output> {
        unimplemented!()
    }

    fn eval_ir(ir: IR) -> Result<Self::Output> {
        let mut code: Vec<u8> = Vec::with_capacity(4096);
        let mut jump_pair_positions: Vec<JumpPairPos> = vec![];

        // Store these functions in registers so I can call them
        // as needed
        let getchar_addr = getchar as usize;
        let putchar_addr = putchar as usize;

        // Iterate over IR instructions, emitting the correct machine code
        // to the code buffer for every instruction. Once we have iterated and
        // emitted all our machine code, buffer should be have all instructions to run
        for ir_insn in ir {
            match ir_insn {
                IRInsn::IncVal(operand) => {
                    code.write_all(&[0x80, 0x01, operand]) // addb $<operand>, (%rcx)
                        .unwrap();
                }

                IRInsn::DecVal(operand) => {
                    code.write_all(&[0x80, 0x29, operand]) // subb $<operand>, (%rcx)
                        .unwrap();
                }

                IRInsn::IncPtr(operand) => {
                    // addq $<operand>, %rcx
                    code.write_all(&[0x48, 0x81, 0xc1]).unwrap();
                    code.write_all(bytemuck::bytes_of(&operand)).unwrap()
                }

                IRInsn::DecPtr(operand) => {
                    // subq $<operand>, %rcx
                    code.write_all(&[0x48, 0x81, 0xe9]).unwrap();
                    code.write_all(bytemuck::bytes_of(&operand)).unwrap();
                }

                IRInsn::JumpIfZero => {
                    /// Compare current pointed to value by loading
                    // its byte in %al, comparing it with zero.
                    code.write_all(&[
                        0x8a, 0x01, // mov %al byte [rcx]
                        0x84, 0xc0, // test %al, %al
                    ])
                    .unwrap();

                    jump_pair_positions.push(JumpPairPos {
                        fwd_jmp: code.len(),
                        bwd_jmp: 0,
                    });

                    code.write_all(&[0x0f, 0x84, 0x0, 0x0, 0x0, 0x0]).unwrap();
                }

                IRInsn::JumpIfNonZero => {
                    // Compare current pointed to value by loading
                    // its byte in %al, comparing it with zero.
                    code.write_all(&[
                        // mov %al byte [rdi]
                        0x8a, 0x01, // test %al, %al
                        0x84, 0xc0, // jnz <0 offset to be patched later>
                    ])
                    .unwrap();

                    jump_pair_positions
                        .iter_mut()
                        .rev()
                        .find(|pair| pair.bwd_jmp == 0)
                        .map(|pair| pair.bwd_jmp = code.len());

                    code.write_all(&[0x0f, 0x85, 0x0, 0x0, 0x0, 0x0]).unwrap();
                }

                IRInsn::GetChar => {
                    // movabsq $<addr of getchar>, %r9
                    code.write_all(&[0x49, 0xb9]).unwrap();
                    code.write_all(bytemuck::bytes_of(&getchar_addr)).unwrap();

                    code.write_all(&[
                        // call *%r9 (getchar)
                        0x41, 0xff, 0xd1, // movb %al, (%rcx)
                        0x88, 0x01,
                    ])
                    .unwrap();
                }

                IRInsn::PutChar => {
                    // movabsq $<addr of getchar>, %r9
                    code.write_all(&[0x49, 0xb9]).unwrap();
                    code.write_all(bytemuck::bytes_of(&putchar_addr)).unwrap();

                    code.write_all(&[
                        // push %rcx
                        0x51, // mov (%rcx), %rcx
                        0x48, 0x8b, 0x09, // call *%r10 (putchar)
                        0x41, 0xff, 0xd1, // pop %rcx
                        0x59,
                    ])
                    .unwrap();
                }
            }
        }

        code.write_all(&[0xc3]).unwrap(); // retq

        jump_pair_positions.into_iter().for_each(|pair| {
            let fwd_offset = (pair.bwd_jmp - pair.fwd_jmp) as i32;
            let bwd_offset = -fwd_offset;

            code[pair.fwd_jmp + 2..pair.fwd_jmp + 6]
                .copy_from_slice(bytemuck::bytes_of(&fwd_offset));

            code[pair.bwd_jmp + 2..pair.bwd_jmp + 6]
                .copy_from_slice(bytemuck::bytes_of(&bwd_offset))
        });

        let mut exec_mem: &mut [u8] = unsafe {
            let ptr =
                Memory::VirtualAlloc(None, code.len(), Memory::MEM_COMMIT, Memory::PAGE_READWRITE);

            let mut _oldflags = Memory::PAGE_PROTECTION_FLAGS(0);

            Memory::VirtualProtect(
                ptr,
                code.len(),
                Memory::PAGE_EXECUTE_READWRITE,
                &mut _oldflags as *mut _,
            )
            .expect("Should be able to enable memory to be executable");

            slice::from_raw_parts_mut(ptr.cast(), code.len())
        };

        // Copy our code inside the dynamically sized vector to the executable memory region
        exec_mem.copy_from_slice(code.as_slice());

        Ok(JittedFunction(exec_mem.as_mut_ptr().cast(), exec_mem.len()))
    }
}
