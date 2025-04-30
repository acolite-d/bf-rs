use super::{
    ir::{IRInsn, IR},
    program::Program,
    Eval,
};

use nix::sys::mman::{mmap_anonymous, munmap, MapFlags, ProtFlags};
use std::{ops::RangeInclusive, ffi::c_void, io::Write, num::NonZero, ptr::NonNull, slice};

// Important Limits for RISC-V
// addi (add immediate) instruction encodes a signed 12-bit number
// define the limits of this value here
const I_FORMAT_IMMED_RANGE: RangeInclusive<i32> = (-2048..=2047);

// Branch instructions also encode a 12-bit signed immediate as a PC-relative offset
// This value is interpreted as a multiple of 2
const B_FORMAT_IMMED_RANGE: RangeInclusive<i32> = (-4096..=4097);

// Bit masks for parts of the immediate 12 bit offset operand for "B" RISC-V instructions
const IMMED_MASK1: i32 = 0b0100_0000_0000; // 11 bit
const IMMED_MASK2: i32 = 0b0000_0000_1111; // bits 1-4
const IMMED_MASK3: i32 = 0b1000_0000_0000; // 12 bit
const IMMED_MASK4: i32 = 0b0011_1111_0000; // bits 5-10

// RISC-V "B" Instruction Format
// Every "-" is a bit in a 4-byte instruction encoding
// |    -    | ------ | ----- | ----- | --- | ---- |    -   | ------- |
// | imm[12] | imm[10:5] | rs2 | rs1 | imm[4:1] | imm[11] | opcode |
//
// This function mutates a "B" format Instructionruction with the desired offset given:
// imm[4:1] + imm[11] and imm[12] + imm[10:5]
fn encode_b_format_immediate_offset(b_format_insn: &mut i32, mut offset: i32) {
    assert!((-4096..4095).contains(&offset)); // +/- 4KB valid range
    assert!(offset % 2 == 0); // has to be divisible by two

    // offset is encoded as multiples of two, so an offset
    // of +8 would be encoded as +4, -12 would be -6, so on.
    // Also, we throw away the least significant bit, since
    // it is asssumed to be zero given we can only branch on
    // multiples of two
    let offset_multiple = offset / 2;

    let imm1 = (offset_multiple & IMMED_MASK1) >> 3;
    let imm2 = (offset_multiple & IMMED_MASK2) << 8;
    let imm3 = (offset_multiple & IMMED_MASK3) << 20;
    let imm4 = (offset_multiple & IMMED_MASK4) << 21;

    let imm = imm1 | imm2 | imm3 | imm4;

    *b_format_insn |= imm;
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
                    // lb t0, (a0)
                    code.write_all(&[0x83, 0x02, 0x05, 0x0]).unwrap();

                    // addi t0, <operand>
                    let add_insn: i32 = 0x00_02_82_93 | (operand as i32) << 20;
                    code.write_all(bytemuck::bytes_of(&add_insn));

                    // sb t0, (a0)
                    code.write_all(&[0x23, 0x0, 0x55, 0x0]).unwrap();
                }

                IRInsn::DecVal(operand) => {
                    // lb t0, (a0)
                    code.write_all(&[0x83, 0x02, 0x05, 0x0]).unwrap();

                    // addi t0, <operand>
                    let sign_ext12 = -(operand as i32) << 20;
                    let add_insn: i32 = 0x00_02_82_93 | sign_ext12;
                    code.write_all(bytemuck::bytes_of(&add_insn));

                    // sb t0, (a0)
                    code.write_all(&[0x23, 0x0, 0x55, 0x0]).unwrap();
                }

                IRInsn::IncPtr(operand) => {
                    // addi a0, <operand>
                    // Possibly many addis are needed to jump around to different
                    // parts of the tape, which is 30,000 bytes in length. The immediate
                    // can only encode a 12-bit sign extended value (-2048..2047)

 //                   if A_FORMAT_IMMED_RANGE.contains(&operand) {
			let sign_ext12: i32 = (operand as i32) << 20;
                        let add_insn: i32 = 0x00_05_05_13 | sign_ext12;
                        code.write_all(bytemuck::bytes_of(&add_insn));
//                    } else {
//                        let adds_needed = operand / (ADDI_IMMEDIATE_MAX as u32);
//                        let rem = operand % (ADDI_IMMEDIATE_MAX as u32);
//                        
//                        let max_addi: i32 = 0x00_05_05_13 | (ADDI_IMMEDIATE_MAX << 20);
//                        
//                        for _ in (0..adds_needed) {
//                            code.write_all(bytemuck::bytes_of(&max_addi)).unwrap();
//                        }
//
//                        if rem > 0 {
//                            let final_addi: i32 = 0x00_05_05_13 | (rem << 20);
//                            code.write_all(bytemuck::bytes_of(&final_addi)).unwrap();
//                        }
                    // }
                }

                IRInsn::DecPtr(operand) => {
                    // if A_FORMAT_IMMED_RANGE.contains(&operand) {
                        // addi a0, -<operand>
                        let sign_ext12 = -(operand as i32) << 20;
                        let add_insn: i32 = 0x00_05_05_13 | sign_ext12;
                        code.write_all(bytemuck::bytes_of(&add_insn));
//                    } else {
//                        let adds_needed = operand / (ADDI_IMMEDIATE_MAX as u32);
//                        let rem = operand % (ADDI_IMMEDIATE_MAX as u32);
//                        
//                        let max_addi: i32 = 0x00_05_05_13 | (I_FORMAT_IMMED_RANGE.start << 20);
//                        
//                        for _ in (0..adds_needed) {
//                            code.write_all(bytemuck::bytes_of(&max_addi)).unwrap();
//                        }
//
//                        if rem > 0 {
//                            let final_addi: i32 = 0x00_05_05_13 | (rem << 20);
//                            code.write_all(bytemuck::bytes_of(&final_addi)).unwrap();
//                        }
                    //}
                }

                IRInsn::JumpIfZero => {
                    // Compare current pointed to value by first loading
                    // its byte to temp register t0

                    // lb t0, (a0)
                    code.write_all(&[0x83, 0x02, 0x05, 0x0]).unwrap(); 

                    // Record where this forward jump is, back patch to its destination later
                    jump_pair_positions.push(JumpPairPos {
                        fwd_jmp: code.len(),
                        bwd_jmp: 0,
                    });
                    
                    // Compare the temp register with loaded value with zero register branch if
                    // equal. The destination is to be backpatched later
                    // beqz t0, x0, 0 (beq t0, x0 (zero register), 0) 
                    code.write_all(&[0x63, 0x80, 0x02, 0x0]).unwrap();
                }

                IRInsn::JumpIfNonZero => {
                    // Compare current pointed to value by first loading
                    // its byte to temp register t0, jump predicated on comparison

                    // lb t0, (a0)
                    code.write_all(&[0x83, 0x02, 0x05, 0x0]).unwrap(); 
                    
                    // Find the last forward jump position that we have written to code buffer
                    // This will be the desitination of backward jump
                    jump_pair_positions
                        .iter_mut()
                        .rev()
                        .find(|pair| pair.bwd_jmp == 0)
                        .map(|pair| {
                            pair.bwd_jmp = code.len();
                        });
                    
                    // Compare the t0 register with the zero register x0, if they are not equal
                    // (meaning t0 is a non-zero value), branch. Destination to be backpatched
                    // later
                    // bnez t0, 0 (bne t0, x0, 0) 
                    code.write_all(&[0x63, 0x90, 0x02, 0x0]).unwrap();
                }

                IRInsn::GetChar => {
                    // A inlined read(63) syscall, read(file_descriptor, buffer, length)
                    // Most of this is putting the right values in registers before making
                    // transfering control to kernel to process read(2)
                    // syscall_number = 0
                    // file_descriptor = STDIN = 0,
                    // buffer = pointer head
                    // length = 1 (single character)
                    code.write_all(&[
                        0x23, 0x3e, 0xa1, 0xfe, // sd a0, -4(sp) (save pointer to stack)
                        0xb3, 0x05, 0x05, 0x0, // add a1, a0, zero
                        0x13, 0x05, 0x0, 0x0, // li a0, 0
                        0x13, 0x06, 0x10, 0x0, // li a2, 0x1 (length)
                        0x93, 0x08, 0xf0, 0x03, // li a7, 63 (syscall number)
                        0x73, 0x0, 0x0, 0x0, // ecall (system call)
                        0x03, 0x35, 0xc1, 0xff, // ld a0, -4(sp) (load it back after syscall)
                    ]).unwrap();
                }

                IRInsn::PutChar => {
                    // A inlined write(64) syscall, write(file_descriptor, buffer, length)
                    // Writes character from pointer head to STDOUT.
                    // file_descriptor = STOUT = 1
                    // syscall number = 1
                    // length = 1 (a single character)
                    code.write_all(&[
                        0x23, 0x3e, 0xa1, 0xfe, // sd a0, -4(sp) (save pointer to stack)
                        0xb3, 0x05, 0x05, 0x0, // add a1, a0, zero
                        0x13, 0x05, 0x10, 0x0, // li a0, 1
                        0x13, 0x06, 0x10, 0x0, // li a2, 0x1 (length)
                        0x93, 0x08, 0x0, 0x04, // li a7, 64 (syscall number)
                        0x73, 0x0, 0x0, 0x0, // ecall (system call)
                        0x03, 0x35, 0xc1, 0xff, // ld a0, -4(sp) (load it back after syscall)
                    ]).unwrap();
                }
            }
        }

        code.write_all(&[0x67, 0x80, 0x0, 0x0]).unwrap(); // ret

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
