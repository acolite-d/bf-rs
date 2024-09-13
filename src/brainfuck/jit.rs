use super::{
    ir::{IRInsn, IR},
    program::Program,
    Eval,
};
use nix::sys::mman::{mmap_anonymous, MapFlags, ProtFlags};
use std::{io::BufWriter, num::NonZero, slice};

pub struct Jit;

impl Eval for Jit {
    type Output = extern "C" fn();

    fn eval_source(src: Program) -> Result<Self::Output, ()> {
        unimplemented!()
    }

    fn eval_ir(ir: IR) -> Result<Self::Output, ()> {
        // Request executable region of memory from operating system using the well-known
        // mmap Linux syscall (see manpages for mmap). This is a Nix API wrapper around said syscall. After we
        // receive the memory, convert it to a mutable slice of bytes (&mut [u8])
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

        let mut pos = 0;

        // With executable region of memory in hand, iterate over IR instructions, emitting the
        // correct machine code to the slice for every instruction. Once we have iterated and
        // emitted all our machine code, exec_mem slice should hold the code to a function.
        for ir_insn in ir {
            match ir_insn {
                IRInsn::IncVal(operand) => {
                    exec_mem[pos..pos + 3].copy_from_slice(&[0x80, 0x07, operand]);
                    pos += 3;
                }

                IRInsn::DecVal(operand) => {
                    exec_mem[pos..pos + 3].copy_from_slice(&[0x80, 0x2f, operand]);
                    pos += 3;
                }

                IRInsn::IncPtr(operand) => {
                    exec_mem[pos..pos + 4].copy_from_slice(&[0x48, 0x83, 0xc7, operand as u8]);
                    pos += 4;
                }

                IRInsn::DecPtr(operand) => {
                    exec_mem[pos..pos + 4].copy_from_slice(&[0x48, 0x83, 0xef, operand as u8]);
                    pos += 4;
                }

                IRInsn::JumpIfZero(dst_offset) => {
                    let dst = (exec_mem.as_ptr().wrapping_add(dst_offset)) as usize;
                }
                IRInsn::JumpIfNonZero(dest) => todo!(),
                IRInsn::GetChar => todo!(),
                IRInsn::PutChar => todo!(),
            }
        }

        // Converting a pointer of bytes to a function pointer in Rust is, as one would expect,
        // very unsafe. This requires an intrinsics function changing arbitrary data types.
        let compiled_fn =
            unsafe { std::mem::transmute::<*const u8, Self::Output>(exec_mem.as_ptr()) };

        Ok(compiled_fn)
    }
}
