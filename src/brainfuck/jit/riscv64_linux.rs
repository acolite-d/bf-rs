use super::{
    ir::{IRInsn, IR},
    program::Program,
    Eval,
};

use nix::sys::mman::{mmap_anonymous, munmap, MapFlags, ProtFlags};
use std::{ffi::c_void, io::Write, num::NonZero, ptr::NonNull, slice};

// Bit masks for parts of the immediate 12 bit offset operand for "B" RISC-V instructions
const IMMED_MASK1: i32 = 0b0100_0000_0000; // 11th bit
const IMMED_MASK2: i32 = 0b0000_0000_1111; // bits 1-4
const IMMED_MASK3: i32 = 0b1000_0000_0000; // 12th bit
const IMMED_MASK4: i32 = 0b0011_1111_0000; // bits 5-10

// RISC-V "B" Instruction Format
// Every "-" is a bit in a 4-byte instruction encoding
// |    -    | ------ | ----- | ----- | --- | ---- |    -   | ------- |
// | imm[12] | imm[10:5] | rs2 | rs1 | imm[4:1] | imm[11] | opcode |
//
// This function mutates a "B" format instruction with the desired offset given:
// imm[4:1] + imm[11] and imm[12] + imm[10:5]
fn encode_b_format_immediate_offset(b_format_insn: &mut i32, offset: i32) {
    assert!((-4095..4096).contains(&offset)); // +/- 4KB valid range
    assert!(offset % 2 == 0); // has to be divisible by two

    // offset is encoded as multiples of two, so an offset
    // of +8 would be encoded as +4, -12 would be -6, so on.
    let offset_multiple = offset / 2;

    let imm1 = (offset & IMMED_MASK1) >> 8;
    let imm2 = (offset & IMMED_MASK2) << 8;
    let imm3 = (offset & IMMED_MASK3) << 20;
    let imm4 = (offset & IMMED_MASK4) << 20;

    *b_format_insn |= (imm1 | imm2 | imm3 | imm4);
}

pub struct Jit;

// The Jit produces JittedFunctions from Brainfuck IR, its a tuple struct
// with a void pointer, and a size of memory pointed to by pointer (weird slice)
pub struct JittedFunction(*mut c_void, usize);

impl JittedFunction {
    pub fn run(&self) {
        // Converting any kind of pointer to a function pointer in Rust is, as one would expect,
        // very unsafe. This requires an intrinsics function changing arbitrary memory objects
        // called "transmute".
        let function =
            unsafe { std::mem::transmute::<*mut c_void, extern "C" fn(*const u8)>(self.0) };

        let byte_arr = [0u8; 30_000];

        // Call the function
        function(byte_arr.as_ptr())
    }
}

// Keeping in touch with Rust's stance on RAII driven design, implemented
// Drop for the JittedFunction object, which call syscall munmap(2) to
// relinquish the executable region of memory we requested from Linux
impl Drop for JittedFunction {
    fn drop(&mut self) {
        unsafe {
            munmap(NonNull::new_unchecked(self.0), self.1)
                .expect("Failed to release memory back to OS!");
        }
    }
}

struct JumpPairPos {
    fwd_jmp: usize,
    bwd_jmp: usize,
}

impl Eval for Jit {
    type Output = JittedFunction;

    fn eval_source(src: Program) -> Result<Self::Output, ()> {
        unimplemented!()
    }

    fn eval_ir(ir: IR) -> Result<Self::Output, ()> {
        let mut code: Vec<u8> = Vec::with_capacity(4096);
        let mut jump_pair_positions: Vec<JumpPairPos> = vec![];

        // Iterate over IR instructions, emitting the correct machine code
        // to the code buffer for every instruction. Once we have iterated and
        // emitted all our machine code, buffer should be have all instructions to run
        for ir_insn in ir {
            match ir_insn {
                IRInsn::IncVal(operand) => {
                    code.write_all(&[0x0, 0x05, 0x02, 0x83]).unwrap(); // lb t0, (a0)

                    let mut addi = 0x28293; // addi t0, t0, 0x0
                    addi |= (operand as i32);

                    bytemuck::bytes_of(&addi)
                        .iter()
                        .rev()
                        .for_each(|&b| code.write_all(&[b]).unwrap());

                    code.write_all(&[0x0, 0x55, 0x0, 0x23]).unwrap(); // sb t0, (a0)
                }

                IRInsn::DecVal(operand) => {
                    code.write_all(&[0x0, 0x05, 0x02, 0x83]).unwrap(); // lb t0, (a0)

                    let mut addi = 0x28293; // addi t0, t0, 0x0
                    addi |= (operand as i32) & 0b1111_1111_1111;

                    bytemuck::bytes_of(&addi)
                        .iter()
                        .rev()
                        .for_each(|&b| code.write_all(&[b]).unwrap());

                    code.write_all(&[0x0, 0x55, 0x0, 0x23]).unwrap(); // sb t0, (a0)
                }

                IRInsn::IncPtr(operand) => {
                    let mut addi = 0x50513; // addi a0, a0, 0x0
                    addi |= (operand as i32);

                    bytemuck::bytes_of(&addi)
                        .iter()
                        .rev()
                        .for_each(|&b| code.write_all(&[b]).unwrap());
                }

                IRInsn::DecPtr(operand) => {
                    let mut addi = 0x50513; // addi a0, a0, 0x0
                    addi |= (operand as i32) & 0b1111_1111_1111;

                    bytemuck::bytes_of(&addi)
                        .iter()
                        .rev()
                        .for_each(|&b| code.write_all(&[b]).unwrap());
                }

                IRInsn::JumpIfZero => {
                    // Compare current pointed to value by first loading
                    // its byte to temp register t0
                    code.write_all(&[0x0, 0x05, 0x02, 0x83]).unwrap(); // lb t0, (a0)

                    jump_pair_positions.push(JumpPairPos {
                        fwd_jmp: code.len(),
                        bwd_jmp: 0,
                    });

                    code.write_all(&[0x0, 0x2, 0x80, 0x63]).unwrap(); // beqz t0, 0
                }

                IRInsn::JumpIfNonZero => {
                    /// Compare current pointed to value by first loading
                    // its byte to temp register t0
                    code.write_all(&[0x0, 0x05, 0x02, 0x83]).unwrap(); // lb t0, (a0)

                    jump_pair_positions
                        .iter_mut()
                        .rev()
                        .find(|pair| pair.bwd_jmp == 0)
                        .map(|pair| {
                            pair.bwd_jmp = code.len();
                        });

                    code.write_all(&[0x0, 0x2, 0x90, 0x63]).unwrap(); // bnez t0, 0
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
                        0xfe, 0xa1, 0x3e, 0x23, // sd a0, -4(sp) (save pointer to stack)
                        0x0, 0x0, 0x5, 0xb7, // lui a1, 0x0 (STDIN)
                        0x0, 0x5, 0x6, 0x33, // add a2, a0, zero (buffer)
                        0x0, 0x0, 0x16, 0xb7, // lui a3, 0x1 (length)
                        0x0, 0x0, 0x5, 0x37, // lui a0, 0x0 (syscall number)
                        0x0, 0x0, 0x0, 0x73, // ecall (system call)
                        0xff, 0xc1, 0x35, 0x3, // ld a0, -4(sp) (load it back after syscall)
                    ])
                    .unwrap();
                }

                IRInsn::PutChar => {
                    // A inlined write(2) syscall, write(file_descriptor, buffer, length)
                    // Writes character from pointer head to STDOUT.
                    // file_descriptor = STOUT = 1
                    // syscall number = 1
                    // length = 1 (a single character)
                    code.write_all(&[
                        0xfe, 0xa1, 0x3e, 0x23, // sd a0, -4(sp) (save our pointer to stack)
                        0x0, 0x0, 0x15, 0xb7, // lui a1, 0x1 (STDIN)
                        0x0, 0x5, 0x6, 0x33, // add a2, a0, zero (buffer)
                        0x0, 0x0, 0x16, 0xb7, // lui a3, 0x1 (length)
                        0x0, 0x0, 0x15, 0x37, // lui a0, 0x0 (syscall number)
                        0x0, 0x0, 0x0, 0x73, // ecall (system call)
                        0xff, 0xc1, 0x35, 0x3, // ld a0, -4(sp) (load it back after syscall)
                    ])
                    .unwrap();
                }
            }
        }

        code.write_all(&[0x0, 0x0, 0x80, 0x67]).unwrap(); // ret

        jump_pair_positions.into_iter().for_each(|pair| {
            let fwd_offset = (pair.bwd_jmp - pair.fwd_jmp) as i32;
            let bwd_offset = -fwd_offset;

            encode_b_format_immediate_offset(
                bytemuck::from_bytes_mut(&mut code[pair.fwd_jmp..pair.fwd_jmp + 4]),
                fwd_offset,
            );

            encode_b_format_immediate_offset(
                bytemuck::from_bytes_mut(&mut code[pair.bwd_jmp..pair.bwd_jmp + 4]),
                bwd_offset,
            );
        });

        // Request executable region of memory from operating system using the well-known
        // mmap Linux syscall (see man pages for mmap). This is a Nix API wrapper around said syscall,
        // where anonymous is just a mapping without a file
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

        // Copy our code inside the dynamically sized vector to the executable memory region
        exec_mem.copy_from_slice(code.as_slice());

        Ok(JittedFunction(exec_mem.as_mut_ptr().cast(), exec_mem.len()))
    }
}
