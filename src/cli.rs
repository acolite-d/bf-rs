use clap::{
    builder::{OsStr, PossibleValue},
    Parser, ValueEnum,
};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// A positional file containing the Brainfuck code you would like to run
    pub file: Option<PathBuf>,

    /// Specifies the mode of execution, Interpret/Just-In-Time Compilation
    #[arg(short, long, value_enum, default_value = Mode::Interpret)]
    pub mode: Mode,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Mode {
    Jit,
    Interpret,
}

impl From<Mode> for OsStr {
    fn from(mode: Mode) -> OsStr {
        match mode {
            Mode::Interpret => "interpreter".into(),
            Mode::Jit => "jit".into(),
        }
    }
}

impl ValueEnum for Mode {
    fn value_variants<'a>() -> &'a [Self] {
        &[Mode::Interpret, Mode::Jit]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(match self {
            Mode::Interpret => PossibleValue::new("interpreter").help("Execute via interpreter"),
            Mode::Jit => {
                PossibleValue::new("jit").help("Execute via Jit compilation and execution")
            }
        })
    }
}
