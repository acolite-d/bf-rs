use anyhow::{anyhow, Result};
use std::collections::HashMap;

use super::error::CompileTimeError;

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Operator {
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
pub struct Program {
    pub code: Box<[Operator]>,
    pub fwd_jump_table: HashMap<usize, usize>,
    pub bwd_jump_table: HashMap<usize, usize>,
}

impl Program {
    pub fn new(source: &str) -> Result<Self> {
        // Define the code as all the valid operators in the file.
        // Anything that is not '>', '<', '+' and so on is a comment
        let operators: Box<[Operator]> = source
            .as_bytes()
            .iter()
            .filter_map(|&byte| byte.try_into().ok())
            .collect();

        // Define jump tables as a pair of hashmaps, one being the inverse of the other.
        // One for forward jumps jumping from '[' to ']', the other vice versa
        // (This is rather expensive way to define jumps but I don't care atm)
        let mut fwd_jump_table: HashMap<usize, usize> = HashMap::new();
        let mut bwd_jump_table: HashMap<usize, usize> = HashMap::new();

        let mut jump_stack = vec![];

        // Using a stack, pop the last '[' location for every ']' to get
        // corresponding brackets that can jump between each other.
        for (offset, operator) in operators.iter().copied().enumerate() {
            match operator {
                Operator::JumpIfZero => jump_stack.push(offset),
                Operator::JumpIfNonZero => {
                    let fwd_jmp_pos = jump_stack
                        .pop()
                        .ok_or(CompileTimeError::UnpairedJumpOperator)?;
                    let bwd_jmp_pos = offset;

                    fwd_jump_table.insert(fwd_jmp_pos, bwd_jmp_pos);
                    bwd_jump_table.insert(bwd_jmp_pos, fwd_jmp_pos);
                }

                _ => {}
            }
        }

        if !jump_stack.is_empty() {
            return Err(anyhow!(CompileTimeError::UnpairedJumpOperator));
        }

        Ok(Self {
            code: operators,
            fwd_jump_table,
            bwd_jump_table,
        })
    }
}

impl IntoIterator for Program {
    type Item = Operator;
    type IntoIter = std::vec::IntoIter<Operator>;

    fn into_iter(self) -> Self::IntoIter {
        self.code.into_vec().into_iter()
    }
}
