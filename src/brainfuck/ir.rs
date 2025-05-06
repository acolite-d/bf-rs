use std::{
    cell::{Ref, RefCell},
    collections::HashMap,
    iter::Peekable,
    ops::Deref,
};

use enum_tag::EnumTag;

use super::program::{Operator, Program};

// Inspiration from Tsoding, https://www.youtube.com/watch?v=mbFY3Rwv7XM
// Same IR really.
#[derive(EnumTag, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum IRInsn {
    IncVal(u8) = 1,
    DecVal(u8) = 2,
    IncPtr(u32) = 3,
    DecPtr(u32) = 4,
    JumpIfZero = 5,
    JumpIfNonZero = 6,
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
            Operator::JumpIfZero => JumpIfZero,
            Operator::JumpIfNonZero => JumpIfNonZero,
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
pub struct IR {
    code: RefCell<Box<[IRInsn]>>,
    pub fwd_jump_table: HashMap<usize, usize>,
    pub bwd_jump_table: HashMap<usize, usize>,
}

impl IR {
    pub fn code(&self) -> Ref<Box<[IRInsn]>> {
        self.code.borrow()
    }
}

impl From<Program> for IR {
    fn from(prog: Program) -> IR {
        let ir: Box<[IRInsn]> = prog.into_iter().map(|op| op.into()).collapse().collect();

        let mut fwd_jump_table: HashMap<usize, usize> = HashMap::new();
        let mut bwd_jump_table: HashMap<usize, usize> = HashMap::new();

        let mut jump_stack = vec![];

        ir.iter()
            .copied()
            .enumerate()
            .for_each(|(offset, insn)| match insn {
                IRInsn::JumpIfZero => jump_stack.push(offset),
                IRInsn::JumpIfNonZero => {
                    let (fwd_dst, bwd_dst) = (jump_stack.pop().unwrap(), offset);
                    fwd_jump_table.insert(fwd_dst, bwd_dst);
                    bwd_jump_table.insert(bwd_dst, fwd_dst);
                }

                _ => {}
            });

        Self {
            code: RefCell::new(ir),
            fwd_jump_table,
            bwd_jump_table,
        }
    }
}

impl IntoIterator for IR {
    type Item = IRInsn;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.code.into_inner().into_vec().into_iter()
    }
}
