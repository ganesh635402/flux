# Flux

**Flux is a modern programming language built from scratch in Rust.**

Flux is an attempt to rethink what a practical programming language can look like when it is designed from the ground up around modern programming needs.

> **Status: Pre-1.0 / Experimental**
>
> Flux is actively being developed. The language and APIs may change before the 1.0 release.

## Why Flux?

Modern software development often requires developers to combine multiple programming models and tools just to build relatively simple systems.

Traditional languages tend to treat things such as:

- time
- events
- asynchronous work
- concurrency
- communication between tasks
- modules and dependencies
- developer tooling

as separate libraries, frameworks, or external abstractions layered on top of the language.

Flux explores a different approach.

### The idea behind Flux

**What if these concepts were considered part of the programming language itself?**

Flux is being built around the idea that a programming language should provide useful primitives for expressing not only computation, but also **time, events, concurrency, communication, and application structure**.

Instead of requiring every problem to be solved through another framework or abstraction layer, Flux aims to make these concepts natural parts of the language.

For example, temporal behavior can be expressed directly:

```flux
after 5s {
    print("Five seconds later!")
}
````

Events can be expressed as language-level behavior:

```flux
on message {
    print("Message received")
}
```

And concurrent execution can be expressed directly:

```flux
let task = spawn {
    print("Running concurrently")
}

await(task)
```

The goal is not to replace every existing programming language.

The goal is to **explore what a programming language designed around these ideas from the beginning could become.**

## What Flux Is Trying to Achieve

Flux is being developed with several core goals:

### Simplicity

Common programming tasks should have straightforward language constructs instead of requiring layers of boilerplate.

### Time as a Language Concept

Time-based behavior should be expressible directly in the language rather than being entirely dependent on external libraries.

### Events as First-Class Behavior

Event-driven programming should feel like a natural programming model rather than a framework imposed on top of the language.

### Concurrency as a Language Primitive

Concurrent execution and communication between concurrent tasks should be directly expressible.

### Application Structure

Modules and packages should be part of the language ecosystem rather than an afterthought.

### 🛠️ Developer Experience

The language itself should provide the tools developers need to work with it:

* Formatter
* Linter
* Test runner
* REPL
* Diagnostics
* Package management
* Developer tooling

### Exploration

Flux is also an experiment.

It is an opportunity to explore language-design questions such as:

> What happens when temporal programming, event-driven programming, concurrency, packages, and developer tooling are designed together instead of independently?

## Features

* Built from scratch in Rust
* Functions, closures, and higher-order functions
* Arrays, maps, structs, and destructuring
* Runtime type system and type checking
* Generics and type aliases
* First-class temporal programming
* Event-driven programming
* Concurrent execution with OS threads
* Channels for communication between tasks
* Modules and package dependencies
* Interactive REPL
* Built-in formatter
* Built-in linter
* Integrated test runner
* Runtime diagnostics and error reporting
* Interactive input support
* Explicit type conversion

## Example

### Hello World

```flux
print("Hello, Flux!")
```

### Interactive Input

```flux
let name = input("What is your name? ")

print("Hello, " + name)
```

### Functions

```flux
fn factorial(n) {
    if n <= 1 {
        return 1
    }

    return n * factorial(n - 1)
}

print(factorial(5))
```

### Temporal Programming

```flux
print("Starting...")

after 5s {
    print("Five seconds later!")
}
```

### Concurrency

```flux
let task = spawn {
    print("Running concurrently")
}

await(task)
```

### Type Conversion

```flux
let age = int("25")
let score = float("98.5")

print(age)
print(score)
print(string(age))
print(bool(age))
```

## Built With

Flux is implemented from scratch in **Rust** and currently uses an interpreter-based execution model.

The project includes:

* Lexer
* Parser
* Abstract Syntax Tree (AST)
* Runtime
* Interpreter
* Standard library
* Module system
* Package manager
* Temporal runtime
* Event system
* Concurrency runtime
* Scheduler
* Formatter
* Linter
* REPL
* Test infrastructure
* Runtime diagnostics

## Documentation

### Language Specification

The complete Flux language specification is available here:

**[Flux Language Specification](docs/spec.md)**

The specification documents the currently supported:

* Syntax
* Lexical rules
* Types
* Variables
* Functions
* Closures
* Control flow
* Arrays
* Maps
* Structs
* Generics
* Destructuring
* Temporal programming
* Events
* Concurrency
* Channels
* Modules
* Packages
* Standard library
* Input
* Type conversion
* Runtime semantics

## Project Status

Flux is currently **pre-1.0 / experimental** and under active development.

The current test suite contains:

**1,905 passing tests**

The project is being developed toward a stable **Flux 1.0** release.

## 🗺️ Roadmap

```text
PHASE 1 ✅ Core Language Completion & Polish
PHASE 2 ✅ Temporal Language Maturity
PHASE 3 ✅ Event-Driven Programming
PHASE 4 ✅ Concurrency
PHASE 5 ✅ Types / Generics / Advanced Language
PHASE 6 ✅ Packages & Ecosystem
PHASE 7 ✅ Developer Tooling
        │
        ▼
     FLUX 1.0
```

## Development

Flux is built using Rust.

Build the project:

```bash
cargo build --release
```

Run the test suite:

```bash
cargo test
```

Check the project:

```bash
cargo check
```

Format the code:

```bash
cargo fmt
```

## Flux CLI

The Flux CLI provides:

```text
flux
flux <file>
flux run <file>
flux check
flux fmt
flux fmt --check
flux test
flux lint
flux repl
flux init
flux deps
flux --version
flux --help
```

Running:

```bash
flux
```

starts the interactive Flux REPL.

## Contributing

Flux is currently an experimental project and is open to exploration, experimentation, feedback, and contributions.

If you find a bug, have an idea, or want to contribute:

1. Open an issue.
2. Describe the problem or proposal.
3. Include a minimal reproduction when applicable.
4. Submit a pull request for changes.

As Flux approaches 1.0, language changes and new features will be evaluated with compatibility and language consistency in mind.

## License

Flux is released under the **MIT License**.

See [LICENSE](LICENSE) for details.

---

## About the Creator

****[Ganesh Rabilli](https://github.com/ganesh635402)**** — Creator and developer of Flux.

**Flux is being built from the ground up — one language feature at a time.**
