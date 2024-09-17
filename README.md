# Another Brainfuck Implementation, Written in Rust, for Exploratory Purposes

Currently only interprets Brainfuck programs, but aim to supply JIT compilation for a number of targets. A number of test programs can be found in the `test_programs/` directory.

To compile, `cargo build`. To run, `cargo run -- --help`. Switch between interpreter and JIT compiler with the "mode" argument.  

## Command Line Interface
```
brainrust$ cargo r -- --help
   Compiling brainrust v0.1.0 (/home/jdorman/projects/brainrust)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.68s
     Running `target/debug/brainrust --help`
Usage: brainrust [OPTIONS] [FILE]

Arguments:
  [FILE]
          A positional file containing the Brainfuck code you would like to run

Options:
  -m, --mode <MODE>
          Specifies the mode of execution, Interpret/Just-In-Time Compilation

          [default: interpreter]

          Possible values:
          - interpreter: Execute via interpreter
          - jit:         Execute via Jit compilation and execution

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

```
```
