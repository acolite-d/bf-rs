pub mod interpreter;
pub mod ir;
pub mod jit;
pub mod program;

pub trait Eval {
    type Output;

    fn eval_source(src: program::Program) -> Result<Self::Output, ()>;

    fn eval_ir(ir: ir::FoldedIR) -> Result<Self::Output, ()>;
}
