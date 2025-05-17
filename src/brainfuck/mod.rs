pub mod interpreter;
pub mod ir;
pub mod jit;
pub mod program;

use anyhow::Result;

pub trait Eval {
    type Output;

    fn eval_source(src: program::Program) -> Result<Self::Output>;

    fn eval_ir(ir: ir::IR) -> Result<Self::Output>;
}
