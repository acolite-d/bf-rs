use thiserror::Error;

#[derive(Error, Debug)]
pub enum CompileTimeError {
    #[error("BAD CODE: Unpaired jump operator detected, every '[' operator needs as ']' to define jump destinations")]
    UnpairedJumpOperator,
}

#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error(
        "RUNTIME ERR: Pointer advanced past last byte of memory, can only address 30,000 bytes"
    )]
    TapeHeadRightOOB,

    #[error("RUNTIME ERR: Pointer advanced past first byte of memory")]
    TapeHeadLeftOOB,
}
