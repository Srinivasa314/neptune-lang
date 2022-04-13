
# Neptune Lang
Neptune is a dynamically typed scripting language

[Documentation](https://docs.rs/neptune-lang/)
[crates.io](https://crates.io/crates/neptune-lang)

TODO: link to book

## Goals
1. Embeddability: It can be embedded in any rust application. Synchronous or asynchronous rust functions can be used by a neptune script.
2. Performance: It performs better than most interpreted scripting languages. See [Benchmarks](BENCHMARKS.md) for a comparison with lua, nodejs and ruby.
3. Security: It is impossible to do any kind of undefined behaviour(including integer overflow) 
4. Concurrency
    * It is trivial to write asynchronous code
    * An error in a task does not always terminate the whole application and can be handled gracefully.
    * There are no function colors.
5. Small implementation

## Features
* Iterators
* String Interpolation
* Modules
* Optional semicolons
* UTF-8 strings
* and much more...

## Getting Started
The CLI be installed by the command given below. You must have the rust compiler and a C++ compiler.
```
cargo install neptune-cli
```
At the moment the CLI has a few basic functions and a REPL. The REPL supports multiline entries and the following shortcuts.
| Command     | Description      |
| ----------- | -----------------|
| Ctrl-L      | Clear the screen |
| Ctrl-C      | Interrupt/Cancel editing |
| Ctrl-D      | Quit REPL                |
| Up arrow    | Previous history entry   |
| Down arrow  | Next history entry       |

To embed it in rust you can use the library from crates.io.


The C++ compiler can be set using the CXX environment variable. Clang is recommended for best performance.

### Todo
* Standard library for the CLI
* Preemptiveness?
* Buffer type