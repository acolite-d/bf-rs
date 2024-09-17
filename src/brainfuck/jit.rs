use super::{
    ir::{IRInsn, IR},
    program::Program,
    Eval,
};

use nix::sys::mman::{mmap_anonymous, MapFlags, ProtFlags};
use std::{
    io::Write,
    num::{NonZero, NonZeroUsize},
    slice,
};

pub struct Jit;

impl Eval for Jit {
    type Output = extern "C" fn(*const u8);

    fn eval_source(src: Program) -> Result<Self::Output, ()> {
        unimplemented!()
    }

    fn eval_ir(ir: IR) -> Result<Self::Output, ()> {
        let mut code: Vec<u8> = Vec::with_capacity(4096);

        // With executable region of memory in hand, iterate over IR instructions, emitting the
        // correct machine code to the slice for every instruction. Once we have iterated and
        // emitted all our machine code, exec_mem slice should hold the code to a function.
        for ir_insn in ir {
            match ir_insn {
                IRInsn::IncVal(operand) => {
                    code.write_all(&[0x80, 0x07, operand]) // addb $<operand>, (%rdi)
                        .unwrap();
                }

                IRInsn::DecVal(operand) => {
                    code.write_all(&[0x80, 0x2f, operand]) // subb $<operand>, (%rdi)
                        .unwrap();
                }

                IRInsn::IncPtr(operand) => {
                    let bytecode: Vec<u8> = {
                        let mut v = vec![0x48, 0x81, 0xc7];
                        v.extend_from_slice(bytemuck::bytes_of(&operand));
                        v
                    }; // addq $<operand>, %rdi

                    code.write_all(bytecode.as_slice()).unwrap();
                }

                IRInsn::DecPtr(operand) => {
                    let bytecode: Vec<u8> = {
                        let mut v = vec![0x48, 0x81, 0xef];
                        v.extend_from_slice(bytemuck::bytes_of(&operand));
                        v
                    }; // subq $<operand>, %rdi

                    code.write_all(bytecode.as_slice()).unwrap();
                }

                IRInsn::JumpIfZero(dest_offset) => {
                    // Compare current pointed to value by loading
                    // its byte in %al, comparing it with zero.
                    code.write_all(&[
                        0x8a, 0x07, // mov %al, byte [rdi]
                        0x84, 0xc0, // test %al, %al
                    ])
                    .unwrap();

                    let jz: Vec<u8> = {
                        let mut v = vec![0x0f, 0x84];
                        v.extend_from_slice(bytemuck::bytes_of(&dest_offset));
                        v
                    }; // jz <rel32_offset_destination>

                    code.write_all(jz.as_slice()).unwrap();
                }

                IRInsn::JumpIfNonZero(dest_offset) => {
                    // Compare current pointed to value by loading
                    // its byte in %al, comparing it with zero.
                    code.write_all(&[
                        0x8a, 0x07, // mov %al byte [rdi]
                        0x84, 0xc0, // test %al, %al
                    ])
                    .unwrap();

                    let jnz: Vec<u8> = {
                        let mut v = vec![0x0f, 0x85];
                        v.extend_from_slice(bytemuck::bytes_of(&dest_offset));
                        v
                    }; // jne <rel32_offset_destination>

                    code.write_all(jnz.as_slice()).unwrap();
                }

                IRInsn::GetChar => {
                    // A inlined read(2) syscall, read(file_descriptor, buffer, length)
                    // Most of this is putting the right values in registers before making
                    // transfering control to kernel to process read(2)
                    // syscall_number = 0
                    // file_descriptor = STDIN = 0,
                    // buffer = pointer head
                    // length = 1 (single character)
                    code.write_all(&[
                        // push %rdi
                        0x57, // mov $0, %rax (syscall number)
                        0x48, 0xc7, 0xc0, 0x0, 0x0, 0x0, 0x0,
                        // mov %rdi, %rsi (second argument, buffer pointer)
                        0x48, 0x89, 0xfe, // mov $0, %rdi (first argument)
                        0x48, 0xc7, 0xc7, 0x0, 0x0, 0x0, 0x0,
                        // mov $1, %rdx (third argument)
                        0x48, 0xc7, 0xc2, 0x01, 0x0, 0x0, 0x0,
                        // syscall, transfer to kernel
                        0x0f, 0x05, // pop %rdi
                        0x5f,
                    ])
                    .unwrap();
                }

                IRInsn::PutChar => {
                    // A inlined write(2) syscall, write(file_descriptor, buffer, length)
                    // Writes character from pointer head to STDOUT.
                    // file_descriptor = STOUT = 1
                    // syscall number = 1
                    // length = 1 (a si ngle character)
                    code.write_all(&[
                        // push %rdi
                        0x57, // mov $1, %rax (syscall number)
                        0x48, 0xc7, 0xc0, 0x01, 0x0, 0x0, 0x0,
                        // mov %rdi, %rsi (second argument, buffer pointer)
                        0x48, 0x89, 0xfe, // mov $1, %rdi (first argument)
                        0x48, 0xc7, 0xc7, 0x01, 0x0, 0x0, 0x0,
                        // mov $1, %rdx (third argument)
                        0x48, 0xc7, 0xc2, 0x01, 0x0, 0x0, 0x0,
                        // syscall, transfer to kernel
                        0x0f, 0x05, // pop %rdi
                        0x5f,
                    ])
                    .unwrap();
                }
            }
        }

        code.write_all(&[0xc3]).unwrap(); // retq

        // Request executable region of memory from operating system using the well-known
        // mmap Linux syscall (see man pages for mmap). This is a Nix API wrapper around said syscall.
        let mut exec_mem: &mut [u8] = unsafe {
            let ptr = mmap_anonymous(
                None,
                NonZero::new_unchecked(code.len()),
                ProtFlags::PROT_READ | ProtFlags::PROT_WRITE | ProtFlags::PROT_EXEC,
                MapFlags::MAP_PRIVATE | MapFlags::MAP_ANONYMOUS,
            )
            .expect("Failed to get executable memory from OS for JIT compilation!")
            .as_ptr()
            .cast();

            slice::from_raw_parts_mut(ptr, code.len())
        };

        exec_mem.copy_from_slice(code.as_slice());

        // Converting a pointer of bytes to a function pointer in Rust is, as one would expect,
        // very unsafe. This requires an intrinsics function changing arbitrary memory objects
        // called "transmute".
        let compiled_fn =
            unsafe { std::mem::transmute::<*const u8, Self::Output>(exec_mem.as_ptr()) };

        Ok(compiled_fn)
    }
}
