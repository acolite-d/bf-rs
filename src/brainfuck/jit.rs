use super::{
    ir::{IRInsn, IR},
    program::Program,
    Eval,
};

use nix::sys::mman::{mmap_anonymous, MapFlags, ProtFlags};
use std::{io::Write, num::NonZero, slice};

pub struct Jit;

impl Eval for Jit {
    type Output = extern "C" fn();

    fn eval_source(src: Program) -> Result<Self::Output, ()> {
        unimplemented!()
    }

    fn eval_ir(ir: IR) -> Result<Self::Output, ()> {
        // Request executable region of memory from operating system using the well-known
        // mmap Linux syscall (see manpages for mmap). This is a Nix API wrapper around said syscall.
        let mut exec_mem: *mut u8 = unsafe {
            mmap_anonymous(
                None,
                NonZero::new_unchecked(4096),
                ProtFlags::PROT_READ | ProtFlags::PROT_WRITE | ProtFlags::PROT_EXEC,
                MapFlags::MAP_PRIVATE | MapFlags::MAP_ANONYMOUS,
            )
            .expect("Failed to get executable memory from OS for JIT compilation!")
            .as_ptr()
            .cast()
        };

        // Convert it to a mutable slice of bytes (&mut [u8]) that has the
        // ergonomic Write trait for writing bytes into buffers, makes things easier to work with
        let mut mem_slice = unsafe { slice::from_raw_parts_mut(exec_mem, 4096) };

        // With executable region of memory in hand, iterate over IR instructions, emitting the
        // correct machine code to the slice for every instruction. Once we have iterated and
        // emitted all our machine code, exec_mem slice should hold the code to a function.
        for ir_insn in ir {
            match ir_insn {
                IRInsn::IncVal(operand) => {
                    mem_slice
                        .write_all(&[0x80, 0x07, operand]) // addb $<operand>, (%rdi)
                        .unwrap();
                }

                IRInsn::DecVal(operand) => {
                    mem_slice
                        .write_all(&[0x80, 0x2f, operand]) // subb $<operand>, (%rdi)
                        .unwrap();
                }

                IRInsn::IncPtr(operand) => {
                    let bytecode: Vec<u8> = {
                        let mut v = vec![0x48, 0x83, 0xe9];
                        v.extend_from_slice(bytemuck::bytes_of(&operand));
                        v
                    };

                    mem_slice.write_all(bytecode.as_slice()).unwrap();
                }

                IRInsn::DecPtr(operand) => {
                    let bytecode: Vec<u8> = {
                        let mut v = vec![0x48, 0x83, 0xe9];
                        v.extend_from_slice(bytemuck::bytes_of(&operand));
                        v
                    };

                    mem_slice.write_all(bytecode.as_slice()).unwrap();
                }

                IRInsn::JumpIfZero(dest_offset) => {
                    let bytecode: Vec<u8> = {
                        let mut v = vec![0xe9];
                        v.extend_from_slice(bytemuck::bytes_of(&dest_offset));
                        v
                    };

                    mem_slice.write_all(bytecode.as_slice()).unwrap();
                }
                IRInsn::JumpIfNonZero(dest_offset) => {
                    let bytecode: Vec<u8> = {
                        let mut v = vec![0xe9];
                        v.extend_from_slice(bytemuck::bytes_of(&dest_offset));
                        v
                    };

                    mem_slice.write_all(bytecode.as_slice()).unwrap();
                }
                IRInsn::GetChar => todo!(),
                IRInsn::PutChar => todo!(),
            }
        }

        // Converting a pointer of bytes to a function pointer in Rust is, as one would expect,
        // very unsafe. This requires an intrinsics function changing arbitrary memory objects
        // called "transmute".
        let compiled_fn = unsafe { std::mem::transmute::<*mut u8, Self::Output>(exec_mem) };

        Ok(compiled_fn)
    }
}
