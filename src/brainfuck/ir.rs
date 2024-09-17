use std::{cell::RefCell, collections::HashMap, iter::Peekable};

use enum_tag::EnumTag;

use super::program::{Operator, Program};

// Inspiration from Tsoding, https://www.youtube.com/watch?v=mbFY3Rwv7XM
// Same IR really.
#[derive(EnumTag, Debug, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum IRInsn {
    IncVal(u8) = 1,
    DecVal(u8) = 2,
    IncPtr(u32) = 3,
    DecPtr(u32) = 4,
    JumpIfZero(i32) = 5,
    JumpIfNonZero(i32) = 6,
    GetChar = 7,
    PutChar = 8,
}

impl IRInsn {
    fn machine_code_size(&self) -> u8 {
        use IRInsn::*;

        match self {
            IncVal(_) | DecVal(_) => 3,
            IncPtr(_) | DecPtr(_) => 7,
            JumpIfZero(_) | JumpIfNonZero(_) => 10,
            GetChar | PutChar => 28,
        }
    }

    fn is_collapsible(&self) -> bool {
        use IRInsn::*;

        matches!(self, IncPtr(_) | DecPtr(_) | IncVal(_) | DecVal(_))
    }

    fn collapse_with(&mut self, other_insn: Self) {
        use IRInsn::*;
        // You should only collapse two instructions to one if
        // they are the same type of instruction!
        assert!(self.tag() == other_insn.tag());

        match (self, other_insn) {
            (IncPtr(x), IncPtr(y)) | (DecPtr(x), DecPtr(y)) => *x = x.wrapping_add(y),

            (IncVal(x), IncVal(y)) | (DecVal(x), DecVal(y)) => *x += y,

            // We can only really collapse increment and decrement instructions
            // on our pointers and memory. Not sure how to optimize jumps or IO
            _ => panic!(),
        }
    }
}

impl From<Operator> for IRInsn {
    fn from(value: Operator) -> Self {
        use IRInsn::*;

        match value {
            Operator::IncrementPtr => IncPtr(1),
            Operator::DecrementPtr => DecPtr(1),
            Operator::IncrementValue => IncVal(1),
            Operator::DecrementValue => DecVal(1),
            Operator::JumpIfZero => JumpIfZero(0),
            Operator::JumpIfNonZero => JumpIfNonZero(0),
            Operator::GetChar => GetChar,
            Operator::PutChar => PutChar,
        }
    }
}

pub struct Collapse<I> {
    iter: I,
}

impl<I> Collapse<I> {
    fn new(iter: I) -> Collapse<I> {
        Self { iter }
    }
}

impl<I> Iterator for Collapse<Peekable<I>>
where
    I: Iterator<Item = IRInsn>,
{
    type Item = IRInsn;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(mut curr_insn) = self.iter.next() {
            while let Some(collapsible) = self
                .iter
                .next_if(|insn| curr_insn.is_collapsible() && curr_insn.tag() == insn.tag())
            {
                curr_insn.collapse_with(collapsible);
            }

            Some(curr_insn)
        } else {
            None
        }
    }
}

pub trait CollapseIR: Iterator<Item = IRInsn> + Sized {
    fn collapse(self) -> Collapse<Peekable<Self>> {
        Collapse::new(self.peekable())
    }
}

impl<I: Iterator<Item = IRInsn>> CollapseIR for I {}

#[derive(Debug)]
pub struct IR(RefCell<Box<[IRInsn]>>);

impl IR {
    pub fn backpatch_jumps(&self) {
        let mut jump_table = HashMap::new();
        let mut fwd_stack = Vec::new();

        self.0
            .borrow()
            .iter()
            .enumerate()
            .for_each(|(pos, insn)| match insn {
                IRInsn::JumpIfZero(_) => {
                    fwd_stack.push(pos);
                }

                IRInsn::JumpIfNonZero(_) => {
                    let last_fwd_pos = fwd_stack.pop().unwrap();

                    jump_table.insert(last_fwd_pos, pos);
                }

                _ => {}
            });

        for (fwd_pos, bwd_pos) in jump_table {
            let jmp_rel_offset: i32 = self
                .0
                .borrow()
                .iter()
                .skip(fwd_pos) // skip to the "[" in question
                .take(bwd_pos - fwd_pos) // iterate over instructions between "[" and "]"
                .map(|insn| insn.machine_code_size() as i32) // As machine code, how many bytes is it?
                .sum();

            let fwd_rel_offset = jmp_rel_offset;
            let bwd_rel_offset = -(jmp_rel_offset);

            if let IRInsn::JumpIfZero(ref mut offset) = self.0.borrow_mut()[fwd_pos] {
                *offset = fwd_rel_offset;
            } else {
                unreachable!();
            }

            if let IRInsn::JumpIfNonZero(ref mut offset) = self.0.borrow_mut()[bwd_pos] {
                *offset = bwd_rel_offset;
            } else {
                unreachable!()
            }
        }
    }
}

impl From<Program> for IR {
    fn from(prog: Program) -> IR {
        let ir = prog.into_iter().map(|op| op.into()).collapse().collect();

        Self(RefCell::new(ir))
    }
}

impl IntoIterator for IR {
    type Item = IRInsn;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_inner().into_vec().into_iter()
    }
}
