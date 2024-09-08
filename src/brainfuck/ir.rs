use std::{collections::HashMap, iter::Peekable};

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
    JumpIfZero(usize) = 5,
    JumpIfNonZero(usize) = 6,
    GetChar = 7,
    PutChar = 8,
}

impl IRInsn {
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
pub struct IR(Box<[IRInsn]>);

impl From<Program> for IR {
    fn from(prog: Program) -> IR {
        // TODO: Go back and add IntoIterator consuming iterator creator for a brainfuck program
        let ir = prog
            .code
            .iter()
            .copied()
            .map(|op| op.into())
            .collapse()
            .collect();

        Self(ir)
    }
}
