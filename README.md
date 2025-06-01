# Nouhjit (脳JIT)
Nouhjit is just another Brainfuck implementation, made just for fun and exploratory purposes. It can both interpret, and JIT compile Brainfuck programs. Meant to be an excuse to learn RISCV assembly.

Currently JIT support is limited to the following platforms:
- x86_64 GNU/Linux
- x86_64 Windows
- RISCV64 GNU/Linux*

**RISCV support includes two versions, one that uses compressed instructions included in the "C" extension of RISCV, and one that does not.**

Some example Brainfuck programs are provided in the test_programs/ directory.

## Command Line Interface
```
Usage: nouhjit [OPTIONS] <FILE>

Arguments:
  <FILE>
          A positional file containing the Brainfuck code you would like to run

Options:
  -m, --mode <MODE>
          Specifies the mode of execution, Interpret/Just-In-Time Compilation

          [default: interpret]

          Possible values:
          - interpret: Execute via interpreter
          - jit:       Execute via Jit compilation and execution

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```
```
```



