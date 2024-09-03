use std::{collections::HashMap, env, ffi::c_int, fs, process};

extern "C" {
    fn getchar() -> c_int;
    fn putchar(char: c_int) -> c_int;
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Operator {
    IncrementPtr = b'>',
    DecrementPtr = b'<',
    IncrementValue = b'+',
    DecrementValue = b'-',
    JumpIfZero = b'[',
    JumpIfNonZero = b']',
    GetChar = b',',
    PutChar = b'.',
}

impl TryFrom<u8> for Operator {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            b'>' => Ok(Self::IncrementPtr),
            b'<' => Ok(Self::DecrementPtr),
            b'+' => Ok(Self::IncrementValue),
            b'-' => Ok(Self::DecrementValue),
            b'[' => Ok(Self::JumpIfZero),
            b']' => Ok(Self::JumpIfNonZero),
            b',' => Ok(Self::GetChar),
            b'.' => Ok(Self::PutChar),
            _ => Err(()),
        }
    }
}

#[derive(Debug)]
struct Program {
    code: Box<[Operator]>,
    fwd_jump_table: HashMap<usize, usize>,
    back_jump_table: HashMap<usize, usize>,
}

impl Program {
    fn new(source: &str) -> Self {
        let operators: Box<[Operator]> = source
            .as_bytes()
            .iter()
            .filter_map(|&byte| byte.try_into().ok())
            .collect();

        let mut fwd_jump_table: HashMap<usize, usize> = HashMap::new();
        let mut back_jump_table: HashMap<usize, usize> = HashMap::new();

        let mut jump_stack = vec![];

        operators
            .iter()
            .copied()
            .enumerate()
            .for_each(|(offset, op)| match op {
                Operator::JumpIfZero => jump_stack.push(offset),
                Operator::JumpIfNonZero => {
                    let (here, there) = (jump_stack.pop().unwrap(), offset);
                    fwd_jump_table.insert(here, there);
                    back_jump_table.insert(there, here);
                }

                _ => {}
            });

        Self {
            code: operators,
            fwd_jump_table,
            back_jump_table,
        }
    }

    fn interpret(&self) {
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
        // execute the right code in every match arm.
        while ip < self.code.len() {
            match self.code[ip] {
                Operator::IncrementPtr => mem_ptr += 1,

                Operator::DecrementPtr => mem_ptr -= 1,

                Operator::IncrementValue => mem[mem_ptr] = mem[mem_ptr].wrapping_add(1),

                Operator::DecrementValue => mem[mem_ptr] = mem[mem_ptr].wrapping_sub(1),

                Operator::JumpIfZero => {
                    if mem[mem_ptr] == 0 {
                        ip = self.fwd_jump_table[&ip]
                    }
                }

                Operator::JumpIfNonZero => {
                    if mem[mem_ptr] != 0 {
                        ip = self.back_jump_table[&ip];
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
    }
}

fn main() {
    // Keep it simple for now, just load the first argument as file,
    // read it as a Brainfuck program, interpret it.
    if let Some(path) = env::args().nth(1) {
        if let Ok(source_code) = fs::read_to_string(&path) {
            let program = Program::new(&source_code);
            program.interpret();
        } else {
            eprintln!("Failed to open file {}", path);
            process::exit(-1)
        }
    } else {
        println!("No args provided.");
        process::exit(-1);
    }
}
