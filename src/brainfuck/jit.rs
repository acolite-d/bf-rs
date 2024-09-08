use super::{ir::IR, program::Program, Eval};
use nix::sys::mman::{mmap_anonymous, MapFlags, ProtFlags};
use std::{num::NonZero, slice};

pub struct Jit;

impl Eval for Jit {
    type Output = extern "C" fn();

    fn eval_source(src: Program) -> Result<Self::Output, ()> {
        unimplemented!()
    }

    fn eval_ir(ir: IR) -> Result<Self::Output, ()> {
        let mut exec_mem: &mut [u8] = unsafe {
            let ptr = mmap_anonymous(
                None,
                NonZero::new_unchecked(4096),
                ProtFlags::PROT_READ | ProtFlags::PROT_WRITE | ProtFlags::PROT_EXEC,
                MapFlags::empty(),
            )
            .expect("Failed to get executable memory from OS for JIT compilation!");

            slice::from_raw_parts_mut(ptr.as_ptr().cast::<u8>(), 4096)
        };

        todo!();
    }
}
