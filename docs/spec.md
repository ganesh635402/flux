# Flux Language Specification

## Overview

Flux is a dynamic, coercive language with modules, functions, arrays, maps,
and a standard library. Programs can be split across multiple `.flux` files.

```
import math

let values = [1, 2, 3, 4, 5]
print(math.sum(values))
print(math.factorial(5))
```

---

## Running Flux Programs

### Basic Usage

```
flux                Start interactive REPL
flux <file>         Run a Flux program
flux run <file>     Run a Flux program
flux --version      Print version information
flux --help         Print help
```

### File Extension

The `.flux` extension is optional when invoking a source file:

```
flux main           runs main.flux
flux main.flux      runs main.flux
flux run main       runs main.flux
flux run main.flux  runs main.flux
```

Relative and absolute paths are supported:

```
flux examples/hello
flux C:\Projects\flux\hello
```

### Exit Codes

| Code | Meaning                                |
| ---- | -------------------------------------- |
| 0    | Successful execution                   |
| 1    | Flux source/runtime/parse/lexing error |
| 2    | CLI usage/argument error               |

---

## Interactive REPL

Running `flux` with no arguments starts an interactive Read-Eval-Print Loop.

```
$ flux
Flux 0.1.0
Interactive mode. Type :help for help, :quit to exit.

>>>
```

### Prompts

- `>>>` — primary prompt, waiting for input
- `...` — continuation prompt, waiting for more lines of a multiline construct

### Expression Results

Bare expressions are automatically evaluated and displayed:

```
>>> 10 + 20
30

>>> "Hello" + " Flux"
Hello Flux

>>> [1, 2, 3]
[1, 2, 3]
```

`print()` output is displayed normally; its `nil` return value is not shown.

### Persistent State

Variables and functions persist across inputs:

```
>>> let x = 10
>>> x + 5
15

>>> fn square(n) { return n * n }
>>> square(7)
49
```

Variables and functions can be redefined.

### Multiline Input

Incomplete constructs (unmatched `{`, `[`, `(`) automatically continue on the next line:

```
>>> fn greet(name) {
...     print("Hello")
...     print(name)
... }
```

### Error Recovery

Errors are displayed but do not terminate the session:

```
>>> undefined_var
<repl>:1:1: error: undefined variable 'undefined_var'
...

>>> 10 + 20
30
```

### REPL Commands

| Command    | Description             |
| ---------- | ----------------------- |
| `:help`    | Show available commands |
| `:clear`   | Reset the environment   |
| `:version` | Show Flux version       |
| `:quit`    | Exit the REPL           |
| `:exit`    | Exit the REPL           |

`:quit` and `:exit` work even during multiline input. EOF (Ctrl+Z on Windows, Ctrl+D on Unix) also exits cleanly.

The language will grow incrementally. This document describes only what is
currently supported.

---

## Lexical Elements

### 1. Identifiers

An identifier starts with an ASCII letter (`a-z`, `A-Z`) or underscore (`_`),
followed by zero or more ASCII letters, digits (`0-9`), or underscores.

**Regex:** `[a-zA-Z_][a-zA-Z0-9_]*`

**Examples:** `print`, `hello`, `_foo`, `my_var2`

**Keywords:** `true`, `false`, `nil`, `let` are identifiers at the lexer level but
treated as keywords by the parser.

### 2. String Literals

A string literal is a sequence of characters enclosed in double quotes (`"`).

- Begins with `"`
- Ends with the next unescaped `"`
- May contain any characters except an unescaped double quote or newline

#### Escape Sequences

The following escape sequences are supported:

| Sequence | Meaning              |
| -------- | -------------------- |
| `\n`     | Newline (line feed)  |
| `\t`     | Horizontal tab       |
| `\r`     | Carriage return      |
| `\\`     | Literal backslash    |
| `\"`     | Literal double quote |
| `\0`     | Null character       |

Escape sequences are validated by the lexer and decoded by the parser.
Invalid escape sequences (e.g., `\x`) produce a lexer error.

```flux
print("hello\nworld")   // prints on two lines
print("col1\tcol2")     // tab-separated
print("path\\to\\file") // literal backslashes
print("she said \"hi\"") // embedded quotes
```

**Examples:** `"Hello, Flux!"`, `""`, `"line1\nline2"`

### 3. Integer Literals

A sequence of ASCII digits.

**Regex:** `[0-9]+`

**Examples:** `0`, `42`, `12345`

Integers are 64-bit signed (`i64`). Arithmetic operations use checked arithmetic:
overflow produces a runtime error rather than wrapping silently.

```flux
print(9223372036854775807 + 1)  // error: integer overflow
print(2 ** 63)                  // error: integer overflow
print(1000000 * 1000000)        // 1000000000000 — within range
```

### 4. Floating-Point Literals

A floating-point literal has an integer part, `.`, and a fractional part.

Leading-dot floats are also valid — a `.` immediately followed by digits:

```flux
3.14
0.5
10.0
.9       // equivalent to 0.9
.25      // equivalent to 0.25
.001     // equivalent to 0.001
```

Scientific notation is not supported.

### 5. Comments

Flux supports single-line and multi-line comments.

**Single-line comments** start with `//` and continue to the end of the line:

```flux
// This is a comment
let x = 10  // trailing comment
```

**Multi-line comments** are enclosed in `/* */`:

```flux
/* This is a
   multi-line comment */
let y = 20
```

Comments are treated as whitespace by the lexer. They can appear between any tokens:

```flux
let x = /* inline comment */ 10
```

Comment syntax inside strings is NOT treated as comments:

```flux
print("// not a comment")
print("/* also not a comment */")
```

Unterminated multi-line comments produce a lexer error. Nested comments are not supported.

### 6. Operators and Punctuation

| Token        | Character |
| ------------ | --------- |
| Plus         | `+`       |
| Minus        | `-`       |
| Star         | `*`       |
| Slash        | `/`       |
| Equals       | `=`       |
| EqualEqual   | `==`      |
| BangEqual    | `!=`      |
| Greater      | `>`       |
| GreaterEqual | `>=`      |
| Less         | `<`       |
| LessEqual    | `<=`      |
| AmpAmp       | `&&`      |
| PipePipe     | `\|\|`    |
| Bang         | `!`       |

`=` is used only in `let` declarations. `==` is used for equality comparison.

Single `&` and `|` are invalid characters.

### 6. Parentheses

`(` and `)` — used for function call syntax and expression grouping.

### 7. Whitespace

Spaces (), tabs (`\t`), carriage returns (`\r`), and newlines (`\n`).

Whitespace separates tokens but is otherwise ignored.

### 8. End of File (EOF)

A synthetic token emitted when the input has been fully consumed.

---

## Value Types

| Type    | Examples           | Description               |
| ------- | ------------------ | ------------------------- |
| String  | `"hello"`, `""`    | UTF-8 text                |
| Integer | `0`, `42`, `12345` | 64-bit signed integer     |
| Float   | `3.14`, `0.5`      | 64-bit floating point     |
| Boolean | `true`, `false`    | Logical truth value       |
| Nil     | `nil`              | Absence of a value; falsy |

---

## Grammar

```
program        = statement* EOF

statement      = let_statement
               | assignment_statement
               | expression_statement
               | if_statement
               | while_statement
               | function_declaration
               | return_statement
               | import_statement

let_statement  = "let" identifier "=" expression
assignment_statement = identifier "=" expression
expression_statement = expression
if_statement   = "if" expression block ("else" block)?
while_statement = "while" expression block
function_declaration = "fn" identifier "(" parameters? ")" block
return_statement = "return" expression?
import_statement = "import" identifier
parameters     = identifier ("," identifier)*
block          = "{" statement* "}"

expression     = logical_or
logical_or     = logical_xor ( "||" logical_xor )*
logical_xor    = logical_and ( "^^" logical_and )*
logical_and    = equality ( "&&" equality )*
equality       = comparison ( ("==" | "!=") comparison )*
comparison     = range ( (">" | "<" | ">=" | "<=") range )*
range          = bitwise_or ( (".." | "..<") bitwise_or )?
bitwise_or     = bitwise_xor ( "|" bitwise_xor )*
bitwise_xor    = bitwise_and ( "^" bitwise_and )*
bitwise_and    = shift ( "&" shift )*
shift          = additive ( ("<<" | ">>") additive )*
additive       = multiplicative ( ("+" | "-") multiplicative )*
multiplicative = power ( ("*" | "/" | "%") power )*
power          = unary ( "**" power )?
unary          = ("!" | "-" | "~") unary | postfix
primary        = STRING | INTEGER | FLOAT | DURATION | "true" | "false" | "nil"
               | member_call | call | identifier
               | "(" expression ")"
               | "[" elements? "]"
               | "{" (expression ":" expression ("," expression ":" expression)*)? "}"
call           = identifier "(" arguments? ")"
member_call    = identifier "." identifier "(" arguments? ")"
arguments      = expression ("," expression)*
elements       = expression ("," expression)*

postfix        = primary ( "[" expression "]" )*
```

---

## Modules

See the [Modules &amp; Imports](#modules--imports) section for the complete module system documentation.

```
// a.flux: import b
// b.flux: import a
// Error: circular module import detected
```

### Missing Modules

```
import nonexistent
// Error: module 'nonexistent' not found
```

### Module Isolation

Each module has its own top-level environment. Variables in one module
do not affect another:

```
// m.flux: let x = 42
// main.flux: let x = 99
// m's x and main's x are independent
```

### Standard Library in Modules

Built-in functions (`print`, `length`, `push`, etc.) are available inside modules.

### Restrictions

- Module names are simple identifiers
- Resolution is relative to the importing file
- No package registry or dependency management

Flux supports `import math as m` (aliases) and `from module import x` (selective imports). See the [Modules &amp; Imports](#modules--imports) section for complete documentation.

## Arrays

### Array Literals

```
[]
[1, 2, 3]
["hello", 42, true, 3.14]
```

Arrays are dynamically typed — elements can be of any type.
Elements are evaluated when the array is created.

### Indexing

```
let numbers = [10, 20, 30]
print(numbers[0])    // 10
print(numbers[2])    // 30
```

Index expressions can be any integer expression:

```
print(numbers[1 + 1])  // 30
```

Inline arrays can be indexed directly:

```
print([10, 20, 30][1])  // 20
```

### Nested Arrays

```
let matrix = [[1, 2], [3, 4]]
print(matrix[0][1])   // 2
print(matrix[1][0])   // 3
```

### Index Rules

- Indices must be Integer values
- Negative indices produce a runtime error
- Out-of-bounds indices produce a runtime error
- Indexing a non-array value produces a runtime error

### Array Truthiness

- `[]` (empty array) → falsy
- Non-empty array → truthy

### Array Display

```
print([1, 2, 3])     // [1, 2, 3]
print(["a", "b"])     // ["a", "b"]
```

### Indexed Assignment

Arrays are mutable. Elements can be replaced by index:

```
let numbers = [10, 20, 30]
numbers[1] = 99
print(numbers[1])    // 99
```

Nested/chained indexed assignment is supported at arbitrary depth:

```
let matrix = [[1, 2], [3, 4]]
matrix[0][1] = 99
print(matrix[0][1])   // 99
```

The assignment target is a general concept — any chain of index operations
rooted at a variable:

```
a[0] = value
a[0][1] = value
a[0][1][2] = value
```

Bounds checking applies at every level — out-of-bounds or negative index
assignment produces a runtime error. Arrays do not auto-grow.

### Reference Semantics

Arrays are reference-like mutable objects. Assignment copies the reference,
not the data:

```
let a = [1, 2, 3]
let b = a
b[0] = 99
print(a[0])    // 99 — a and b share the same array
```

Functions receive array references — mutations inside functions are visible
to the caller:

```
fn change(values) {
    values[0] = 100
}
let numbers = [1, 2, 3]
change(numbers)
print(numbers[0])   // 100
```

Primitive values (Integer, Float, Boolean, String) remain value-like.

### length()

Built-in function that returns the number of elements:

```
print(length([10, 20, 30]))   // 3
print(length([]))              // 0
print(length("hello"))         // 5
print(length({"a": 1}))       // 1
```

Unsupported types produce a runtime error:

```
length(42)   // error: length not supported for Integer
```

---

## Maps

### Map Literals

```
{}
{"name": "Flux", "version": 1}
{1: "one", 2: "two"}
```

Maps are ordered key-value collections. Keys must be String or Integer.
Values can be any type. Entries maintain insertion order.

### Map Access

```
let person = {"name": "Ron", "age": 25}
print(person["name"])    // Ron
print(person["age"])     // 25
```

Missing keys return `nil`:

```
print(person["missing"])   // nil
```

### Map Mutation

```
person["age"] = 26
print(person["age"])    // 26
```

New keys can be added:

```
person["email"] = "ron@flux.dev"
```

### Nested Access and Mutation

Maps integrate with the generalized indexing system:

```
let data = {"user": {"name": "Alice"}}
data["user"]["name"] = "Ron"
print(data["user"]["name"])   // Ron
```

Mixed nesting with arrays:

```
let data = {"users": [{"name": "Alice"}]}
data["users"][0]["name"] = "Ron"
```

### Reference Semantics

Maps use reference semantics like arrays:

```
let a = {"x": 10}
let b = a
b["x"] = 20
print(a["x"])    // 20
```

### Map Truthiness

- `{}` (empty map) → falsy
- Non-empty map → truthy

### Map Display

```
print({"name": "Flux", "version": 1})
// {"name": "Flux", "version": 1}
```

### keys()

Built-in function that returns an array of a map's keys:

```
let m = {"a": 1, "b": 2}
print(keys(m))    // ["a", "b"]
```

---

## Standard Library

### Output

- `print(value)` — print any value

### Input

- `input()` — read one line from stdin, return as String
- `input(prompt)` — print prompt (without newline), then read one line

### Type System

- `type(value)` — returns type name as String: `"Integer"`, `"Float"`, `"Boolean"`, `"String"`, `"Array"`, `"Map"`, `"Nil"`
- `is_nil(value)`, `is_number(value)`, `is_string(value)`, `is_boolean(value)`, `is_array(value)`, `is_map(value)` — type predicates returning Boolean

### Conversions

- `int(value)` — explicit conversion to Integer:
  - Integer -> unchanged
  - Float -> truncated toward zero (must be finite and in `i64` range)
  - Boolean -> `true` => `1`, `false` => `0`
  - String -> parsed as signed base-10 integer after trimming whitespace
  - Otherwise: runtime error
- `float(value)` — explicit conversion to Float:
  - Float -> unchanged
  - Integer -> converted to Float
  - Boolean -> `true` => `1.0`, `false` => `0.0`
  - String -> parsed as finite decimal float after trimming whitespace
  - Otherwise: runtime error
- `string(value)` — explicit conversion to String using normal display formatting
- `bool(value)` — explicit conversion to Boolean using Flux truthiness rules

Conversions are explicit; operators do not perform additional implicit type coercion beyond their defined operand rules.

### String Operations

- `upper(string)` — uppercase
- `lower(string)` — lowercase
- `trim(string)` — strip leading/trailing whitespace
- String `+` String — concatenation: `"Hello " + "Flux"` → `"Hello Flux"`

### Collection Operations

- `length(value)` — length of Array, String, or Map
- `keys(map)` — array of map keys
- `push(array, value)` — append to array (mutates)
- `pop(array)` — remove and return last element (mutates)
- `contains(array, value)` — check if array contains value
- `contains_key(map, key)` — check if map has key
- `remove_key(map, key)` — remove entry, returns removed value or nil

### Math

- `abs(value)` — absolute value
- `min(a, b)` — minimum of two numeric values
- `max(a, b)` — maximum of two numeric values
- `floor(value)` — floor to integer
- `ceil(value)` — ceiling to integer
- `round(value)` — round to nearest integer

### Nil Semantics

- `nil == nil` → `true`
- `nil == 0` → `false`; `nil == false` → `false`; `nil == ""` → `false`
- `nil` is falsy (`!nil` → `true`)

### Collection Equality

- Arrays and Maps use identity (reference) equality
- `[1, 2] == [1, 2]` → `false` (different array instances)
- `let a = [1]; let b = a; a == b` → `true` (same reference)

---

## Functions

### Declaration

```
fn name(params) {
    statements
}
```

Function declarations are registered before execution (hoisted), allowing
forward references and mutual recursion.

### Calls

Function calls are expressions that evaluate to the return value:

```
add(10, 20)          // standalone call
let x = add(10, 20)  // call in expression
print(add(2, 3) * 4) // call in arithmetic
```

Arguments are evaluated left-to-right and passed by value.

### Return

```
return expression    // returns a value
return               // returns nil
```

`return` immediately exits the function, even through nested `if`/`while` blocks.
`return` outside a function is a runtime error.

### Local Scope

Functions have their own local environment. Local variables do not leak:

```
let x = 10
fn test() {
    let x = 20    // local to test()
    print(x)      // 20
}
test()
print(x)          // 10
```

Functions can read and mutate global variables:

```
let counter = 0
fn increment() {
    counter = counter + 1
}
```

### Parameters

Parameters are local variables bound to argument values:

```
fn add(a, b) {
    return a + b
}
```

Duplicate parameter names produce a parse error.

### Argument Count

Argument count must match parameter count:

```
fn add(a, b) { return a + b }
add(10)        // error: expected 2 argument(s) but got 1
add(10, 20, 30) // error: expected 2 argument(s) but got 3
```

### Recursion

Functions can call themselves recursively:

```
fn factorial(n) {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}
```

A configurable recursion depth limit (default: 256) prevents stack overflow.

### Nil

`return` without a value and functions without `return` produce `nil`.
`nil` is falsy and displays as `nil`.

### Restrictions

- No variadic arguments

Flux supports anonymous function expressions, closures, function values, higher-order functions, and closure environment capture. See the later sections on closures and function values for details.

---

## Control Flow

### if / else

```
if condition {
    statements
}

if condition {
    statements
} else {
    statements
}
```

The condition is any expression. Its result is converted to a boolean using
Flux truthiness rules. If truthy, the then-block executes. Otherwise, the
else-block executes (if present).

Examples:

```
let score = 85
if score >= 90 {
    print("Excellent")
} else {
    print("Good")
}
```

### Blocks

Blocks are delimited by `{` and `}` and contain zero or more statements.
They execute sequentially.

**Important:** Blocks do NOT currently introduce lexical scope. Variables
declared inside a block are visible after the block. This will change when
functions and proper scoping are introduced.

```
if true {
    let x = 42
}
print(x)    // works — no lexical scope yet
```

### Truthiness in conditions

All value types can be used as conditions:

| Condition        | Result                         |
| ---------------- | ------------------------------ |
| `true`           | executes then-block            |
| `false`          | executes else-block (or skips) |
| `0`              | falsy                          |
| nonzero          | truthy                         |
| `0.0`            | falsy                          |
| nonzero float    | truthy                         |
| `""`             | falsy                          |
| non-empty string | truthy                         |

### Unexecuted branches

Statements in unexecuted branches are not evaluated:

```
if false {
    print(undefined_var)   // no error — never reached
}
```

### Nested if

```
if x > 0 {
    if x < 100 {
        print("between 0 and 100")
    }
}
```

---

## while Loops

```
while condition {
    statements
}
```

The condition is evaluated before each iteration using Flux truthiness.
If truthy, the body executes and the condition is re-evaluated.
If falsy, the loop stops.

Examples:

```
let x = 0
while x < 5 {
    print(x)
    x = x + 1
}
```

### Dynamic conditions

All value types can be used as while conditions via truthiness:

```
while 1 { ... }      // truthy — loops (use with caution)
while 0 { ... }      // falsy — never executes
while "hello" { ... } // truthy
while "" { ... }      // falsy — never executes
```

### Loop iteration safety limit

The interpreter enforces a configurable loop iteration limit (default:
1,000,000) to prevent accidental infinite loops from hanging the process.
When exceeded, a runtime error is produced:

```
runtime error: loop iteration limit exceeded
```

This is an interpreter safety mechanism, not a language semantic.

---

## Variables

### Declaration

Variables are declared with `let`:

```
let x = 10
let name = "Flux"
let result = 10 + 20 * 3
```

### Mutability and Assignment

Variables can be reassigned after declaration:

```
let x = 10
x = 20        // assignment — updates existing variable
print(x)      // 20
```

Assignment is a statement, NOT an expression. `print(x = 10)` is invalid.

Declaration (`let x = 10`) and assignment (`x = 20`) are distinct:

- `let` creates a new binding; duplicate `let` is an error
- `=` updates an existing binding; assigning to an undefined variable is an error

Variables are dynamically typed — a variable may hold different types over
its lifetime:

```
let x = 10
x = "hello"
x = true
```

### Variable lookup

Variables are available after their declaration:

```
let x = 10
print(x)        // works
let y = x + 5   // x is available here
```

### Initialization

The initializer is evaluated before the variable is stored. Self-references
during initialization produce an error:

```
let x = x + 1   // error: undefined variable 'x'
```

### Forward references

Variables cannot be used before they are declared:

```
print(x)         // error: undefined variable 'x'
let x = 10
```

### Duplicate declarations

Redeclaring a variable in the same scope is an error:

```
let x = 10
let x = 20       // error: variable 'x' is already defined
```

### Reserved names

Keywords (`true`, `false`, `let`, `if`, `else`, `while`) cannot be used as
variable names.

---

## Operator Precedence

From lowest to highest:

| Level | Operators           | Associativity  |
| ----- | ------------------- | -------------- |
| 1     | `\|\|`              | Left           |
| 2     | `^^`                | Left           |
| 3     | `&&`                | Left           |
| 4     | `==` `!=`           | Left           |
| 5     | `>` `<` `>=` `<=`   | Left           |
| 6     | `..` `..<` (range)  | —              |
| 7     | `\|` (bitwise OR)   | Left           |
| 8     | `^` (bitwise XOR)   | Left           |
| 9     | `&` (bitwise AND)   | Left           |
| 10    | `<<` `>>`           | Left           |
| 11    | `+` `-`             | Left           |
| 12    | `*` `/` `%`         | Left           |
| 13    | `**`                | Right          |
| 14    | `!` `-` `~` (unary) | Right (prefix) |

Parentheses override precedence: `(10 + 20) * 3` evaluates to `90`.

Examples:

- `2 + 3 > 4` means `(2 + 3) > 4`
- `true || false && false` means `true || (false && false)`
- `!false && true` means `(!false) && true`
- `-2 * 3` means `(-2) * 3`
- `10 - -5` means `10 - (-5)`
- `2 ** 3 ** 2` means `2 ** (3 ** 2)` = 512 (right-associative)
- `2 * 3 ** 2` means `2 * (3 ** 2)` = 18

### Unary Operators

| Operator | Meaning                  | Operand Types           |
| -------- | ------------------------ | ----------------------- |
| `!`      | Logical NOT (truthiness) | Any                     |
| `-`      | Numeric negation         | Integer, Float, Boolean |
| `~`      | Bitwise NOT              | Integer, Boolean        |

### Arithmetic Operators

| Operator | Meaning                  | Notes                                              |
| -------- | ------------------------ | -------------------------------------------------- |
| `+`      | Addition / string concat | String + String also supported                     |
| `-`      | Subtraction              |                                                    |
| `*`      | Multiplication           |                                                    |
| `/`      | Division                 | Integer division truncates; div by zero → error    |
| `%`      | Modulo                   | Remainder; mod by zero → error; Float uses Rust`%` |
| `**`     | Exponentiation           | Right-associative; negative exponent → Float       |

### Bitwise Operators

| Operator | Meaning                  | Operand Types                     |
| -------- | ------------------------ | --------------------------------- |
| `&`      | Bitwise AND              | Integer, Boolean (coerced to 0/1) |
| `\|`     | Bitwise OR               | Integer, Boolean                  |
| `^`      | Bitwise XOR              | Integer, Boolean                  |
| `~`      | Bitwise NOT              | Integer, Boolean                  |
| `<<`     | Left shift               | Integer; shift count must be 0–63 |
| `>>`     | Right shift (arithmetic) | Integer; shift count must be 0–63 |

Float operands produce a runtime error for bitwise operations.

### Logical Operators

| Operator | Meaning     | Short-circuit                      |
| -------- | ----------- | ---------------------------------- |
| `&&`     | Logical AND | Yes: skips right if left is falsy  |
| `\|\|`   | Logical OR  | Yes: skips right if left is truthy |
| `^^`     | Logical XOR | No: both operands always evaluated |

All logical operators return Boolean values based on truthiness.

### Compound Assignment

```
+=  -=  *=  /=  %=  &=  |=  ^=  <<=  >>=
```

`x op= expr` is equivalent to `x = x op expr`. Works with variables and indexed targets (`arr[i] += 5`).

### Negation Details

`-` as a prefix operator negates numeric values:

```flux
-10       // -10
-3.14     // -3.14
-true     // -1 (boolean coerced to integer, then negated)
-false    // 0
-x        // negates the value of x
-(10 + 5) // -15
```

Negating non-numeric types (String, Array, Map, Function, Range, Nil) produces a runtime error.

Integer overflow when negating `i64::MIN` produces a runtime error.

---

## Arithmetic Semantics

### Numeric coercion

Values of type Integer, Float, and Boolean can participate in arithmetic.
Booleans are coerced to integers: `true → 1`, `false → 0`.

```
Integer + Integer → Integer
Integer + Float → Float
Float + Integer → Float
Float + Float → Float
Boolean + Integer → Integer  (true→1, false→0)
Integer * Boolean → Integer
Boolean + Boolean → Integer
2.5 * true → Float
```

Examples:

```
print(true + 10)    → 11
print(false + 10)   → 10
print(true * 5)     → 5
print(true + true)  → 2
```

### Disallowed operations

String values cannot participate in arithmetic other than concatenation (`+`):

```
"hello" + 10        → error: cannot apply '+' to String and Integer
42 - "hello"        → error: cannot apply '-' to Integer and String
```

---

## Division by Zero

Division by zero produces a runtime error. Boolean coercion applies first:

```
print(10 / 0)       → Runtime error: division by zero
print(1.0 / 0.0)   → Runtime error: division by zero
print(10 / false)   → Runtime error: division by zero  (false → 0)
```

---

## Dynamic and Coercive Type Semantics

Flux values retain their runtime types (String, Integer, Float, Boolean),
but the language performs implicit coercion when an operation has a
well-defined conversion.

### Truthiness

Every value has a boolean interpretation:

| Value            | Truthy? |
| ---------------- | ------- |
| `false`          | false   |
| `true`           | true    |
| `0`              | false   |
| nonzero Integer  | true    |
| `0.0`            | false   |
| nonzero Float    | true    |
| `""`             | false   |
| non-empty String | true    |

### Boolean → Number

When a Boolean participates in arithmetic or comparison:

- `true` → `1`
- `false` → `0`

### Number → Boolean

When a numeric value is used in a boolean context (logical operators):

- `0` / `0.0` → false
- nonzero → true

### String Truthiness

Strings participate in boolean contexts through emptiness:

- `""` → false
- non-empty → true

Strings do NOT implicitly convert to numbers for arithmetic.

---

## Comparison Semantics

Comparison operators (`>`, `<`, `>=`, `<=`) work on numeric-coercible values
(Integer, Float, Boolean):

```
Integer compared with Integer → Boolean
Float compared with Float → Boolean
Integer compared with Float → Boolean (promote Integer to Float)
Boolean compared with Integer → Boolean (true→1, false→0)
```

Examples:

```
print(true > false)   → true   (1 > 0)
print(true > 2)       → false  (1 > 2)
print(false <= 0)     → true   (0 <= 0)
```

Strings cannot be compared for ordering:

```
"a" > "b"     → error: cannot apply '>' to String and String
```

---

## Equality Semantics

Equality (`==`, `!=`) uses numeric coercion for Integer, Float, and Boolean:

```
10 == 10.0     → true   (numeric promotion)
true == 1      → true   (true→1)
false == 0     → true   (false→0)
true == 1.0    → true   (true→1→1.0)
true == 42     → false  (1 != 42)
```

String equality is string-to-string only:

```
"hello" == "hello"  → true
"hello" == "world"  → false
```

Cross-category equality (String with any numeric type) is an error:

```
"1" == 1       → error: cannot apply '==' to String and Integer
"true" == true → error: cannot apply '==' to String and Boolean
```

---

## Logical Operators

`&&`, `||`, and `!` use truthiness. They always return `Boolean`.

```
print(1 && 2)        → true   (both truthy)
print(0 && 2)        → false  (0 is falsy)
print(0 || 42)       → true   (42 is truthy)
print(!0)            → true   (0 is falsy)
print(!42)           → false  (42 is truthy)
print(!"")           → true   (empty string is falsy)
print(!"hello")      → false  (non-empty string is truthy)
```

---

## Short-Circuit Evaluation

`&&` and `||` use short-circuit evaluation based on truthiness:

- If the left side of `&&` is falsy → result is `false`, right not evaluated
- If the left side of `||` is truthy → result is `true`, right not evaluated

```
print(0 && undefined_var)   → false  (right side never evaluated)
print(42 || undefined_var)  → true   (right side never evaluated)
```

---

## Built-in Functions

### `print(expression)`

Evaluates the expression and outputs the result followed by a newline.

Display rules:

- Strings: content without quotes
- Integers: decimal representation
- Floats: decimal with at least one decimal place (e.g. `10.0`, `3.14`)
- Booleans: `true` or `false`

---

### `input()` / `input(prompt)`

Reads one line from standard input and returns it as a `String`.

#### Forms

```flux
input()              // read a line, no prompt
input("Name: ")      // print prompt, then read a line
```

#### Behavior

1. If a prompt argument is given, it is printed to stdout **without** a trailing newline, and stdout is flushed.
2. One line is read from stdin.
3. The trailing line ending (`\n` or `\r\n`) is removed.
4. The result is returned as a `String`.

#### Return Type

Always returns `String`, even if the user enters numeric or boolean-looking text:

```flux
let x = input()    // user enters "123"
print(type(x))     // "String"
```

Use `int()` or `float()` to convert if needed.

#### Empty Input

Pressing Enter with no text returns `""` (empty string), not `nil`:

```flux
let x = input()    // user presses Enter
print(type(x))     // "String"
print(length(x))   // 0
```

#### Whitespace Preservation

Only the trailing line ending is removed. Leading/trailing spaces are preserved:

```flux
let x = input()    // user enters "  hello  "
print(x)           // "  hello  "
```

#### EOF Handling

If standard input reaches EOF before a line is read, `input()` throws a runtime error:

```
input: unexpected end of input
```

This can be caught with `try`/`catch`:

```flux
try {
    let x = input()
} catch e {
    print(e)    // "input: unexpected end of input"
}
```

#### Argument Errors

- `input("a", "b")` → arity error: `input expects 0 or 1 argument(s) but got 2`
- `input(42)` → type error: `input prompt must be String, got Integer`
- `input(nil)` → type error: `input prompt must be String, got Nil`

#### REPL Behavior

`input()` works inside the interactive REPL. The REPL reads its own command lines separately; `input()` reads the next line from stdin during expression evaluation:

```
>>> let name = input("Name: ")
Name: Ron
>>> print(name)
Ron
```

#### File Execution

`input()` works when executing `.flux` files. Standard input is read from the process stdin:

```flux
// greet.flux
let name = input("What is your name? ")
print("Hello " + name)
```

```
$ flux greet.flux
What is your name? Ron
Hello Ron
```

#### Concurrency

In spawned tasks (`spawn { ... }`), `input()` reads from the process stdin. Concurrent calls to `input()` from multiple spawned tasks may interleave unpredictably. For deterministic behavior, use `input()` only on the main thread.

#### Testing

The interpreter provides an `Input` trait and `TestInput` implementation for deterministic testing without interactive stdin. Tests inject pre-loaded input lines via `set_input()`.

---

### `int(value)`, `float(value)`, `string(value)`, `bool(value)`

Explicit type-conversion built-ins:

- `int(value)`
  - Integer -> unchanged
  - Float -> truncated toward zero
  - Boolean -> `true` => `1`, `false` => `0`
  - String -> parse signed decimal integer after trimming whitespace
  - Errors on invalid literals, out-of-range values, and unsupported types
- `float(value)`
  - Float -> unchanged
  - Integer -> converted to float
  - Boolean -> `true` => `1.0`, `false` => `0.0`
  - String -> parse finite decimal float after trimming whitespace
  - Errors on invalid literals and unsupported types
- `string(value)`
  - Converts any value to its display string
- `bool(value)`
  - Converts any value using Flux truthiness

Examples:

```flux
print(int("  -42  "))      // -42
print(float(" 2.5 "))      // 2.5
print(string([1, 2, 3]))    // [1, 2, 3]
print(bool(""))             // false
print(bool("0"))            // true
```

Flux favors explicit conversion. If a value must change type, call one of these built-ins directly.

---

## Unsupported Characters

Any character that does not begin a valid token is an error. The lexer
reports the character and its position and continues scanning.

---

## Valid Examples

```flux
print("Hello, Flux!")
print(42)
print(3.14)
print(true)
print(10 + 20)
print(10 + 20 * 3)
print((10 + 20) * 3)
print(10 + 2.5)
let x = 10
let y = x + 20
print(y)
print(x > 5)
print(x == 10)
print(x != 20)
print(true && false)
print(true || false)
print(!false)
print(x > 5 && x < 20)
print(true + 10)
print(true == 1)
print(1 && 2)
print(!0)
print(!"")
```

---

## Invalid Examples

```flux
print("unterminated)
```

> String literal is never closed.

```flux
print("hello" + "world")
```

> Prints `helloworld`. String concatenation uses the `+` operator.

```flux
print(10 / 0)
```

> Division by zero runtime error.

```flux
print(x)
```

> Undefined variable error (if `x` was not declared).

```flux
let x = 10
let x = 20
```

> Duplicate variable declaration error.

```flux
@invalid
```

> `@` is not a valid character.

---

## Token Summary Table

| Token Kind      | Pattern                          | Example          |
| --------------- | -------------------------------- | ---------------- |
| Identifier      | `[a-zA-Z_][a-zA-Z0-9_]*`         | `print`          |
| StringLiteral   | `"` … `"`                        | `"Hello, Flux!"` |
| IntegerLiteral  | `[0-9]+`                         | `42`             |
| FloatLiteral    | `[0-9]+.[0-9]+`                  | `3.14`           |
| DurationLiteral | `[0-9]+(ns\|us\|ms\|s\|m\|h\|d)` | `5s`, `100ms`    |
| LeftParen       | `(`                              | `(`              |
| RightParen      | `)`                              | `)`              |
| Plus            | `+`                              | `+`              |
| Minus           | `-`                              | `-`              |
| Star            | `*`                              | `*`              |
| Slash           | `/`                              | `/`              |
| Equals          | `=`                              | `=`              |
| EqualEqual      | `==`                             | `==`             |
| BangEqual       | `!=`                             | `!=`             |
| Greater         | `>`                              | `>`              |
| GreaterEqual    | `>=`                             | `>=`             |
| Less            | `<`                              | `<`              |
| LessEqual       | `<=`                             | `<=`             |
| AmpAmp          | `&&`                             | `&&`             |
| PipePipe        | `\|\|`                           | `\|\|`           |
| Bang            | `!`                              | `!`              |
| EOF             | (end of input)                   |                  |

Whitespace is consumed but not emitted as a token.

---

## 21. Destructuring & Collection Patterns

### Pattern Concept

Flux distinguishes _expressions_ from _patterns_:

- **Expression**: produces a Value (e.g., `1 + 2`, `[1, 2]`, `f()`)
- **Pattern**: describes how a Value is unpacked and bound (e.g., `[a, b]`, `{"name": name}`)

Patterns are used in `let` declarations, destructuring assignments, function parameters, and `for` loops.

### Pattern Types

```
Pattern
 ├── Identifier(name)   — bind to a variable
 ├── Wildcard (_)        — discard the value
 ├── Array([patterns])   — destructure an array
 └── Map([(key, pattern)]) — destructure a map
```

### Array Destructuring

```flux
let [a, b, c] = [10, 20, 30]
// a = 10, b = 20, c = 30
```

**Strict length matching**: the number of pattern elements must exactly match the array length. Mismatches produce a runtime error.

```flux
let [a, b] = [1]       // error: expected 2, received 1
let [a] = [1, 2]       // error: expected 1, received 2
```

Empty array patterns are allowed: `let [] = []` succeeds, `let [] = [1]` fails.

### Map Destructuring

```flux
let {"name": name, "age": age} = {"name": "Ron", "age": 25}
// name = "Ron", age = 25
```

Map pattern keys must be **string literals**. Each key is looked up in the map value.

- **Missing keys** produce a runtime error: `key "name" not found during destructuring`
- **Extra keys** in the map are allowed and ignored.

### Nested Patterns

Patterns can nest arbitrarily:

```flux
let [first, [second, third]] = [10, [20, 30]]
let {"user": {"name": name}} = {"user": {"name": "Flux"}}
let [{"name": n1}, {"name": n2}] = [{"name": "A"}, {"name": "B"}]
```

### Wildcard Pattern

`_` discards a value without creating a binding:

```flux
let [first, _, third] = [10, 20, 30]
// first = 10, third = 30, _ is not defined
```

Using `_` as a variable reference is an error (undefined variable).

### Duplicate Bindings

Duplicate names within a pattern are rejected at parse time:

```flux
let [x, x] = [1, 2]           // parse error: duplicate binding 'x'
let {"a": x, "b": x} = data   // parse error: duplicate binding 'x'
fn f(x, x) {}                 // parse error: duplicate parameter name 'x'
```

### Destructuring in Declarations

`let` supports patterns as the binding target:

```flux
let [a, b] = get_values()
let {"name": name} = get_person()
```

The RHS is evaluated exactly once before binding.

### Destructuring Assignment

Assign into existing variables using patterns:

```flux
let x = 0
let y = 0
[x, y] = [100, 200]
// x = 100, y = 200
```

All target variables must already exist. Attempting to assign to an undefined variable produces a runtime error.

**Atomic semantics**: if any part of the pattern fails (length mismatch, type mismatch, undefined variable), no bindings are modified. Values are extracted and validated first, then assigned.

### Function Parameter Patterns

Function parameters can be patterns:

```flux
fn greet([name, age]) {
    print(name)
    print(age)
}
greet(["Ron", 25])

fn show({"name": name}) {
    print(name)
}
show({"name": "Flux"})
```

Nested patterns in parameters are supported. Destructuring failures during function calls produce runtime errors with call stack information.

### For-Loop Patterns

The loop variable can be a pattern:

```flux
for [x, y] in [[1, 2], [3, 4], [5, 6]] {
    print(x + y)
}
// 3, 7, 11
```

Each iteration creates a fresh scope and binds the pattern against the current element.

### `entries()` Standard Library Function

Returns an array of `[key, value]` pairs from a map, preserving insertion order:

```flux
let m = {"a": 1, "b": 2}
for [key, value] in entries(m) {
    print(key)
    print(value)
}
// a, 1, b, 2
```

### Type Restrictions

- Array patterns only match `Array` values. Other types (Integer, String, Range, Nil, etc.) produce errors.
- Map patterns only match `Map` values.
- Ranges are NOT automatically destructured as arrays.
- Strings are NOT automatically treated as character arrays.

### Scope Semantics

Destructured variables follow the same scoping rules as normal `let` declarations. Closures correctly capture destructured bindings.

### Not Supported

- Rest patterns (`[first, ...rest]`)
- Default parameter values
- Literal patterns for matching
- `match` expressions
- Arbitrary expressions as map pattern keys

---

## Temporal Model

Flux treats time as a first-class concept with two core types: `Instant` and `Duration`.

### Instant

Represents a specific point on the Flux clock (monotonic, nanosecond resolution).

```flux
let start = now()
print(type(start))   // Instant
```

### Duration

Represents an amount of elapsed time, stored as nanoseconds (signed, integer-based).

```flux
let d = 5s
print(d)              // 5s
print(type(d))        // Duration
```

### Duration Literals

Duration values can be written directly using numeric literals with a time-unit suffix:

| Suffix | Unit         | Example |
| ------ | ------------ | ------- |
| `ns`   | nanoseconds  | `100ns` |
| `us`   | microseconds | `500us` |
| `ms`   | milliseconds | `250ms` |
| `s`    | seconds      | `5s`    |
| `m`    | minutes      | `1m`    |
| `h`    | hours        | `2h`    |
| `d`    | days         | `3d`    |

The suffix must immediately follow the integer digits (no space). Only integer bases are supported — no floating-point duration literals.

```flux
let timeout = 5s
sleep(100ms)
after 2s { print("done") }
every 10s { print(now()) }
```

Duration literals produce the same values as the constructor functions: `5s == seconds(5)`.

### Duration Constructor / Extraction Functions

These functions serve dual purpose:

- **With Integer argument**: create a Duration (`seconds(5)` → `5s`)
- **With Duration argument**: extract total count in that unit (`seconds(90s)` → `90`)

| Function          | Create mode                | Extract mode                  |
| ----------------- | -------------------------- | ----------------------------- |
| `nanoseconds(n)`  | Duration from nanoseconds  | Total nanoseconds as Integer  |
| `microseconds(n)` | Duration from microseconds | Total microseconds as Integer |
| `milliseconds(n)` | Duration from milliseconds | Total milliseconds as Integer |
| `seconds(n)`      | Duration from seconds      | Total seconds as Integer      |
| `minutes(n)`      | Duration from minutes      | Total minutes as Integer      |
| `hours(n)`        | Duration from hours        | Total hours as Integer        |
| `days(n)`         | Duration from days         | Total days as Integer         |

Extraction uses integer division (truncates toward zero):

```flux
seconds(90s + 500ms)   // 90 (truncated)
milliseconds(5s)        // 5000
hours(1d)               // 24
```

### Duration Display

Durations display in the largest whole unit: `5s`, `500ms`, `2m`, `1h`, `1d`, `0s`.

### Duration Arithmetic

```flux
5s + 2s     // 7s
5s - 2s     // 3s
5s * 3      // 15s
3 * 5s      // 15s
10s / 2     // 5s
1m + 30s    // 90s
```

Negative durations are supported: `seconds(-5)` → `-5s`.

### Instant Arithmetic

```flux
Instant + Duration → Instant
Instant - Duration → Instant
Instant - Instant  → Duration
Duration + Instant → Instant
```

Invalid operations (`Instant + Instant`, `Instant * 2`, `Duration + Integer`) produce runtime errors.

### Temporal Comparisons

Durations and Instants support `==`, `!=`, `<`, `<=`, `>`, `>=`:

```flux
seconds(5) < seconds(10)   // true
seconds(5) == seconds(5)   // true
```

### now()

Returns the current `Instant` from the Flux clock.

### sleep(duration)

Blocks execution for the given Duration. Negative durations produce a runtime error.

```flux
sleep(milliseconds(100))
```

### Truthiness

- `Duration(0)` → false
- `Duration(nonzero)` → true
- `Instant` → always true

### Temporal Utility Functions

| Function           | Description                                               |
| ------------------ | --------------------------------------------------------- |
| `elapsed(instant)` | Duration since`instant` (equivalent to `now() - instant`) |
| `since(instant)`   | Alias for`elapsed(instant)`                               |
| `between(a, b)`    | Duration from Instant`a` to Instant `b` (`b - a`)         |

```flux
let start = now()
// ... some work ...
let d = elapsed(start)     // Duration since start
let d2 = since(start)      // same as elapsed
let d3 = between(a, b)     // b - a
```

### Duration Predicates

| Function         | Description              |
| ---------------- | ------------------------ |
| `is_zero(d)`     | true if Duration is zero |
| `is_negative(d)` | true if Duration < 0     |
| `is_positive(d)` | true if Duration > 0     |

### Instant Predicates

| Function       | Description                  |
| -------------- | ---------------------------- |
| `is_past(t)`   | true if Instant`t` < `now()` |
| `is_future(t)` | true if Instant`t` > `now()` |

```flux
is_zero(0s)            // true
is_negative(-5s)       // true
is_positive(5s)        // true
is_future(now() + 1h)  // true
```

### Clock Abstraction

The interpreter uses a `Clock` trait for `now()` and a `Sleeper` trait for `sleep()`.
Production uses `SystemClock`/`SystemSleeper`. Tests use `TestClock`/`TestSleeper` for deterministic behavior without real waiting.

### Overflow Safety

All temporal arithmetic uses checked operations. Overflow produces a runtime error, never a panic.

---

## Temporal Scheduling

### `after`

Schedules a block to execute after a specified duration or at a specific instant. Non-blocking — program execution continues immediately.

```flux
print("start")

after 1s {
    print("later")
}

print("end")
```

Output: `start`, `end`, `later`

The delay expression must evaluate to a `Duration` or `Instant`.

- **Duration**: schedules relative to `now()`. Zero delay is valid (executes on first scheduler tick). Negative delay is an error.
- **Instant**: schedules at that absolute time.

```flux
let deadline = now() + 30s
after deadline {
    print("deadline reached")
}
```

The program waits for all pending `after` tasks to complete before exiting.

### `every`

Schedules a block to execute repeatedly at a fixed interval.

```flux
every seconds(1) {
    print("tick")
}
```

The interval must be a positive `Duration`. Zero or negative intervals produce a runtime error.

### Non-Blocking Semantics

`after` and `every` are scheduling operations, not blocking sleep calls. Code after the scheduling statement executes immediately.

### Environment Capture

Scheduled blocks capture the lexical environment (like closures). They see mutations made before execution:

```flux
let x = 10
after seconds(1) { print(x) }
x = 20
// Prints 20 (sees the mutation)
```

### Ordering

Tasks due at the same time execute in FIFO order (scheduling order). Each task has a monotonically increasing ID.

### Recurring Timing

`every` uses non-overlapping execution:

```
next_run = completion_time + interval
```

This prevents backlog accumulation.

### Nested Scheduling

Scheduled blocks can schedule more work:

```flux
after seconds(1) {
    print("first")
    after seconds(1) {
        print("second")
    }
}
```

### Error Handling

Errors in scheduled callbacks are reported but do not terminate other scheduled tasks. The scheduler continues executing remaining tasks.

### Scheduler Architecture

The interpreter maintains a `Scheduler` that holds pending tasks. After program execution completes, `run_scheduler()` processes pending tasks using the `Clock`/`Sleeper` abstractions. Tests use `TestClock` + `scheduler_tick()` for deterministic execution without real waiting.

### Future Temporal Features (Not Yet Implemented)

- Timezone-aware scheduling

---

## Calendar Time

Flux provides calendar/wall-clock types separate from the monotonic `Instant` system.

### Temporal Type Hierarchy

| Type       | Purpose                                  |
| ---------- | ---------------------------------------- |
| `Instant`  | Monotonic machine timeline (from`now()`) |
| `Duration` | Elapsed amount of time                   |
| `Date`     | Calendar date (year-month-day)           |
| `Time`     | Time of day (hour:minute:second)         |
| `DateTime` | Wall-clock timestamp (date + time)       |

### Constructors

```flux
date(year, month, day)              // Date
time(hour, minute)                  // Time
time(hour, minute, second)          // Time
time(hour, minute, second, nano)    // Time
datetime()                          // current DateTime
datetime(year, month, day, hour, minute)        // DateTime
datetime(year, month, day, hour, minute, second) // DateTime
```

### Accessors

```flux
year(d)      month(d)      day(d)       // Date or DateTime
hour(t)      minute(t)     second(t)    // Time or DateTime
weekday(d)                              // Date or DateTime → String ("Monday", etc.)
days_in_month(d)                        // Date or DateTime → Integer
is_leap_year(year_or_date)              // Integer, Date, or DateTime → Boolean
```

### Display

```
Date:     2026-08-30
Time:     14:30:15
DateTime: 2026-08-30 14:30:15
```

### Arithmetic

| Operation             | Result           |
| --------------------- | ---------------- |
| `Date + Duration`     | Date (day-level) |
| `Date - Duration`     | Date             |
| `Date - Date`         | Duration         |
| `DateTime + Duration` | DateTime         |
| `DateTime - Duration` | DateTime         |
| `DateTime - DateTime` | Duration         |

Time arithmetic is not supported in this stage to avoid ambiguous day-boundary semantics.

### Comparisons

Same-type comparisons (`<`, `<=`, `>`, `>=`, `==`, `!=`) are supported for Date, Time, and DateTime. Cross-type comparisons produce runtime errors.

### Validation

Invalid dates/times produce runtime errors:

```flux
date(2026, 2, 30)   // error: invalid day
date(2026, 13, 1)   // error: invalid month
time(25, 0)          // error: invalid hour
```

Leap years are correctly handled: `date(2024, 2, 29)` is valid, `date(2025, 2, 29)` is not.

### Truthiness

Date, Time, and DateTime are always truthy.

### Wall Clock Abstraction

`datetime()` uses a `WallClock` trait. Production uses `SystemWallClock`. Tests use `TestWallClock` for deterministic behavior.

### Distinction: `now()` vs `datetime()`

- `now()` returns `Instant` — monotonic elapsed-time measurement
- `datetime()` returns `DateTime` — human calendar timestamp

These are fundamentally different types and cannot be mixed.

### Timezone Limitation

Stage 25 provides UTC-based wall-clock time. Timezone-aware types, conversion, and DST handling are deferred to a future stage.

---

## Task Lifecycle

Scheduling constructs (`after`, `every`, `at`) return first-class `Task` handles.

### Task Handles

```flux
let task = after seconds(5) { print("done") }
let task = every seconds(1) { print("tick") }
let task = at datetime(2026, 9, 1, 9, 0) { print("event") }
```

Tasks have identity-based equality: two tasks are equal only if they reference the same scheduled work.

### Task States

| State       | Description                        |
| ----------- | ---------------------------------- |
| `pending`   | Waiting to execute                 |
| `running`   | Currently executing                |
| `completed` | One-shot task finished             |
| `cancelled` | Cancelled (will not execute again) |

### Cancellation

```flux
let task = every seconds(1) { print("tick") }
cancel(task)
```

Cancellation is idempotent — calling `cancel()` multiple times is safe.

Self-cancellation is supported:

```flux
let count = 0
let task = every seconds(1) {
    count += 1
    if count >= 5 { cancel(task) }
}
```

Cancelled recurring tasks are not rescheduled. Cancelled tasks do not keep the program alive.

### Task Introspection

```flux
is_cancelled(task)   // Boolean
is_done(task)        // Boolean (true if completed or cancelled)
is_running(task)     // Boolean (true only during callback execution)
```

Tasks can be stored in variables, arrays, maps, and passed to functions.

### Awaitable Tasks

Tasks can produce results that are retrieved with `await`:

```flux
let task = after seconds(1) {
    return 42
}

let result = await task
print(result)    // 42
```

`await` blocks until the task completes and returns its result. A task that completes without `return` produces `nil`.

#### Multiple awaits

The same task can be awaited multiple times — the result is reusable:

```flux
let a = await task
let b = await task    // same result
```

#### Task errors

If a task's callback throws, `await` re-throws the error:

```flux
let task = after seconds(1) { throw "boom" }
try {
    await task
} catch e {
    print(e)    // "boom"
}
```

#### Cancelled tasks

Awaiting a cancelled task produces a catchable error:

```flux
cancel(task)
await task    // error: task was cancelled
```

#### Recurring tasks

Recurring tasks (`every`, calendar schedules) cannot be awaited:

```flux
let task = every seconds(1) { ... }
await task    // error: cannot await a recurring task
```

### Error Handling

- `cancel(non_task)` → runtime error
- `is_cancelled(non_task)` → runtime error
- `is_running(non_task)` → runtime error
- Callback runtime errors cancel recurring tasks and are reported but do not crash the scheduler

---

## Calendar-Aware Scheduling

### `at datetime(...)`

Schedules a one-shot block at a specific wall-clock time:

```flux
at datetime(2026, 9, 1, 9, 0) { print("event") }
```

If the target time has already passed, the task executes on the next scheduler tick.

### `at time(...)`

Schedules for a specific time-of-day:

```flux
at time(9, 0) { print("morning") }
```

- If today's time has not passed: schedules for today
- If today's time has passed: schedules for tomorrow

### Duration vs Calendar Scheduling

| Construct          | Clock Used       | Target Type            |
| ------------------ | ---------------- | ---------------------- |
| `after seconds(5)` | Monotonic`Clock` | `Duration` → `Instant` |
| `every seconds(5)` | Monotonic`Clock` | `Duration` interval    |
| `at datetime(...)` | `WallClock`      | `DateTime` → `Instant` |
| `at time(...)`     | `WallClock`      | `Time` → `Instant`     |

### Deferred Features

- `cron`, timezone support
- Task groups, parallel execution
- Calendar literal syntax

---

## Temporal Control Flow

### `until` Loop

Executes a body while a condition is falsy. Stops when the condition becomes truthy.

```flux
let count = 0
until count >= 5 {
    print(count)
    count = count + 1
}
// Prints: 0, 1, 2, 3, 4
```

Uses Flux truthiness rules. `break`, `continue`, and `return` work as in `while`/`for`.

**Safety limit:** `until false { }` (infinite loop without body work) is protected by the iteration limit (same as `while`).

### `wait until`

Temporal suspension — waits for a condition to become truthy without busy-spinning.

```flux
wait until ready
```

Uses scheduler polling (10ms intervals) to re-evaluate the condition. Pending scheduled tasks (`after`/`every`) are executed during the wait, allowing them to change the condition.

### `wait until` with Timeout

```flux
wait until condition timeout seconds(10)
```

If the condition doesn't become truthy within the timeout duration, a runtime error is produced:

```
wait until timed out
```

The timeout uses the monotonic `Clock`, not wall-clock time.

### Distinction

| Construct                  | Purpose                                       |
| -------------------------- | --------------------------------------------- |
| `until condition { body }` | Repeated execution while condition is falsy   |
| `wait until condition`     | Temporal suspension until condition is truthy |

`until` is a loop — it executes code repeatedly.
`wait until` is a wait — it suspends execution.

### Deadline Pattern

```flux
let deadline = now() + seconds(30)
wait until now() >= deadline
// or
wait until completed timeout seconds(30)
```

---

## Calendar Recurrence

Calendar-based recurring schedules run at specific calendar times rather than elapsed durations.

### Daily

```flux
every day at time(9, 0) {
    print("Good morning")
}
```

### Weekly

```flux
every Monday at time(9, 0) {
    print("Monday meeting")
}
```

Supported weekday names: `Monday`, `Tuesday`, `Wednesday`, `Thursday`, `Friday`, `Saturday`, `Sunday`.

### Monthly

```flux
every month on 15 at time(9, 0) {
    print("Monthly report")
}
```

**Month-end normalization:** If the requested day exceeds the month's length, the last valid day is used (e.g., `on 31` in February → February 28/29).

### Yearly

```flux
every year on 12/25 at time(9, 0) {
    print("Christmas")
}
```

**Leap year:** `on 2/29` in non-leap years executes on February 28.

### Duration vs Calendar Recurrence

| Construct                        | Clock            | Meaning                    |
| -------------------------------- | ---------------- | -------------------------- |
| `every seconds(5) { ... }`       | Monotonic`Clock` | Every 5 elapsed seconds    |
| `every day at time(9,0) { ... }` | `WallClock`      | At 09:00 each calendar day |

These are fundamentally different scheduling models. Duration recurrence uses elapsed time; calendar recurrence uses wall-clock dates.

### Scheduling Semantics

- If current time is before target time today → first execution is today
- If current time is after target time today → first execution is the next matching day
- Missed occurrences are skipped (no catch-up bursts)
- All calendar tasks return `Task` handles and support `cancel()`

### Deferred Features

- `every day at ...` as cron equivalent
- Timezone support
- Task groups, parallel execution

---

## Modules & Imports

### Basic Import

```flux
import math           // loads math.flux
import utils.math     // loads utils/math.flux (nested module)
```

The `.flux` extension is implicit. Modules are resolved relative to the importing file's directory.

### Module Namespace

Imported modules expose their exports via dot notation:

```flux
import math

print(math.pi)         // variable access
print(math.square(5))  // function call
```

### Import Aliases

```flux
import utils.math as m
print(m.square(5))
```

### Selective Import

```flux
from math import square
from math import square, cube
from math import square as sq
```

Selective imports bind directly into the importing scope without requiring dot notation.

### Private Bindings

Names starting with `_` are private to the module:

```flux
// In module:
let _internal = 42
fn _helper() { ... }

// From outside:
math._internal    // error: cannot access private binding
```

### Module Initialization

Top-level code executes once on first import. Subsequent imports reuse the cached module.

### Module Caching

Modules are cached by canonical file path. Multiple import paths to the same file share one module instance.

### Circular Imports

Detected and produce a clear error: `circular module import detected: ...`

### Module Scope

Each module has its own environment. Functions retain access to their module's globals (closures work across modules).

### Module State

Module-level variables persist. Functions in a module access and modify the module's own variables:

```flux
// counter.flux
let _count = 0
fn increment() {
    _count += 1
    return _count
}

// main.flux
import counter
print(counter.increment())  // 1
print(counter.increment())  // 2
```

### Module Isolation

Importing a module does not affect the importing scope's variables:

```flux
let PI = 999
import math       // math has its own PI = 3.14159
print(PI)         // 999
print(math.PI)    // 3.14159
```

### Relative Module Resolution

Modules are resolved relative to the importing file's directory, not the working directory. Nested modules: `import utils.math` resolves to `utils/math.flux`.

### Missing Module/Export Errors

```flux
import nonexistent     // error: module 'nonexistent' not found
math.does_not_exist    // error: module 'math' has no export 'does_not_exist'
```

### REPL

```
>>> import math
>>> math.square(5)
25
>>> from math import square
>>> square(5)
25
```

---

## Error Handling

### Error Values

Errors are first-class values created with `error(message)`:

```flux
let e = error("something failed")
print(type(e))     // Error
print(e)           // something failed
```

Errors are truthy. Errors with the same message are equal.

### `throw`

Raises a Flux error that propagates through the call stack:

```flux
throw "something went wrong"
throw error("bad input")
```

Strings are automatically wrapped in Error values when thrown.

### `try` / `catch` / `finally`

```flux
try {
    risky_operation()
} catch e {
    print(e)          // handle the error
} finally {
    cleanup()         // always runs
}
```

- `catch` captures the error and stops propagation
- `finally` always runs (success, error, return, break, continue)
- `try` requires at least `catch` or `finally`

### Error Propagation

Errors propagate through function calls until caught:

```flux
fn inner() { throw "failure" }
fn outer() { inner() }

try { outer() } catch e { print(e) }   // "failure"
```

### Control Flow Distinction

`catch` only catches errors — **not** `return`, `break`, or `continue`:

```flux
fn test() {
    try { return 10 } catch e { print("never runs") }
}
// return propagates normally
```

### `finally` Semantics

- Runs after try body regardless of outcome
- If `finally` throws, its error replaces the original error/signal
- `return` inside try still propagates after `finally` executes

### Runtime Errors Are Catchable

Division by zero, undefined variables, destructuring failures, and timeouts become catchable:

```flux
try { print(10 / 0) } catch e { print(e) }          // "division by zero"
try { wait until false timeout seconds(0) } catch e { print(e) }  // "wait until timed out"
```

### Scheduled Callback Errors

Errors in `after`/`every` callbacks are caught by the scheduler — they don't crash the program. Recurring tasks with errors are cancelled to prevent infinite error loops.

### Uncaught Errors

Uncaught errors produce a clean diagnostic with source location and exit code 1.

### Cancellation Is Not an Error

`cancel(task)` is task lifecycle control, not an exception.

---

## Event Model

Flux provides a first-class `Event` value type representing occurrences that can carry data.

### Event Values

An Event has three components:

| Component   | Type      | Description                                    |
| ----------- | --------- | ---------------------------------------------- |
| `type`      | String    | Identifies what happened (e.g.`"message"`)     |
| `payload`   | any Value | Data associated with the event (default:`nil`) |
| `timestamp` | Instant   | When the event was created (from the clock)    |

### Event Constructor

```flux
event("message")                          // nil payload
event("message", "hello")                 // string payload
event("temperature", 25)                  // integer payload
event("user.created", {"id": 42})         // map payload
event("deadline", now() + 5s)             // instant payload
```

- First argument must be a String (event type). Non-string produces a runtime error.
- Second argument is optional — omitted means `nil` payload.
- Any Flux value is a valid payload, including other Events.
- Timestamp is automatically assigned from the interpreter's clock (deterministic in tests via `TestClock`).

### Event Accessors

| Function        | Returns   | Description                    |
| --------------- | --------- | ------------------------------ |
| `event_type(e)` | String    | The event's type/name          |
| `event_data(e)` | any Value | The event's payload            |
| `event_time(e)` | Instant   | The event's creation timestamp |

All accessors require an Event argument; other types produce a runtime error.

### Equality

Events use **structural equality** — same type and same payload:

```flux
event("x", 1) == event("x", 1)    // true
event("x", 1) != event("x", 2)    // true
event("x", 1) != event("y", 1)    // true
```

Timestamp is excluded from equality comparison. Two events with the same type and payload are equal regardless of when they were created.

### Truthiness

Events are always truthy, regardless of payload:

```flux
if event("x") { print("truthy") }          // truthy
if event("x", nil) { print("truthy") }     // truthy
```

### Type

```flux
type(event("x"))    // "Event"
```

### Display

Events display as `Event("type", payload)`:

```flux
print(event("msg", "hi"))       // Event("msg", "hi")
print(event("tick"))            // Event("tick", nil)
print(event("n", 42))           // Event("n", 42)
```

### Errors

```flux
event()                // error: event expects 1 or 2 argument(s) but got 0
event(42)              // error: event type must be String, got Integer
event_type(42)         // error: event_type expects Event, got Integer
```

### Event Emission

Events are submitted to the runtime's event queue using `emit()`:

```flux
emit(event("message", "hello"))
emit(event("tick"))
emit(event("data", [1, 2, 3]))
```

`emit()` accepts exactly one Event argument. Non-Event values produce a runtime error. Returns `nil`.

Emitted events are stored in FIFO order in the interpreter's event queue. They retain their original timestamp and payload unchanged.

### Event Queue

The event queue is an internal FIFO buffer. Events emitted first are processed first.

| Function         | Returns      | Description                             |
| ---------------- | ------------ | --------------------------------------- |
| `event_count()`  | Integer      | Number of events currently in the queue |
| `last_event()`   | Event or nil | Most recently emitted event, or nil     |
| `pop_event()`    | Event or nil | Remove and return the oldest event      |
| `clear_events()` | nil          | Remove all events from the queue        |

### Event Handlers

Handlers are registered with the `on` statement:

```flux
on "message" {
    print("received a message")
}

on "message" as e {
    print(event_data(e))
}

on "temperature" as e where event_data(e) > 30 {
    print("hot!")
}
```

**Syntax**: `on <type_expr> [as <param>] [where <filter>] { body }`

- `type_expr` must evaluate to a String (the event type to match)
- `as param` optionally binds the matched Event to a variable
- `where filter` optionally filters events (handler skipped if filter is falsy)
- Handlers execute in registration order
- Handler bodies execute in a child scope of the captured environment
- Errors inside handlers are isolated — they do not crash the program
- Multiple handlers can match the same event type

### Event Dispatch

The `dispatch()` function processes all queued events through matching handlers:

```flux
on "tick" { print("tock") }
emit(event("tick"))
dispatch()              // prints "tock", returns 1
```

- Returns the number of events dispatched (Integer)
- Events are consumed from the queue in FIFO order
- Each event is matched against all active handlers by type
- Handlers that emit new events during dispatch will have those events processed in the same dispatch cycle
- Unmatched events are consumed but ignored

### Handler Lifecycle

| Function             | Args    | Returns | Description                          |
| -------------------- | ------- | ------- | ------------------------------------ |
| `handler_count()`    | 0       | Integer | Number of active handlers            |
| `cancel_handler(id)` | Integer | Boolean | Cancel handler by ID (true if found) |
| `off(event_type)`    | String  | Integer | Cancel all handlers for a type       |

```flux
on "x" { print("X") }          // handler ID 0
cancel_handler(0)               // returns true
off("x")                        // cancels all "x" handlers
```

Cancelled handlers are never invoked. Cancellation is idempotent.

### Event + Temporal Integration

Events integrate naturally with the temporal scheduler:

```flux
on "tick" { print("tock") }

every 1s {
    emit(event("tick"))
}
```

```flux
on "timeout" { print("timed out") }

after 5s {
    emit(event("timeout"))
}
```

The `run_scheduler()` loop automatically dispatches events after each scheduler tick. The `process()` builtin does one cycle of: scheduler tick + event dispatch.

| Function    | Returns | Description                                            |
| ----------- | ------- | ------------------------------------------------------ |
| `process()` | Boolean | Run one scheduler+dispatch cycle. true if work remains |

### Emission Errors

```flux
emit()                 // error: emit expects 1 argument(s) but got 0
emit(42)               // error: emit expects Event, got Integer
on 42 { }              // error: on expects event type as String, got Integer
```

### Deferred Event Features (Not Yet Implemented)

- Concurrent/parallel handlers
- Channels and actors
- Async execution
- Network events

---

## Concurrency Model

Flux uses a **hybrid concurrency model**:

- **Temporal scheduling** (`after`, `every`, `at`) executes cooperatively on the main interpreter thread via the scheduler.
- **`spawn`** creates real OS threads with deep-cloned, isolated environments using `Arc<Mutex<...>>` for thread-safe task and channel state.

### Key Distinctions

| Concept                  | Purpose                   | Mechanism                               |
| ------------------------ | ------------------------- | --------------------------------------- |
| **Sequential execution** | Normal statements         | Interpreter executes in order           |
| **Scheduling**           | _When_ should work happen | `after`, `every`, `at` + Scheduler      |
| **Events**               | _What_ happened           | `event()`, `emit()`, `on`, `dispatch()` |
| **Concurrency**          | Independent progress      | `spawn` creates real OS threads         |

Scheduling determines _when_ something executes. Concurrency determines _whether_ independent computations can make progress without waiting for each other. These are separate concepts.

### Execution Model

The interpreter uses `Rc<RefCell<...>>` for local state. When `spawn` creates a new task, the environment is deep-cloned into a `SendableTaskPayload` that can safely be moved to an OS thread. `FluxTask` and `FluxChannel` use `Arc<Mutex<...>>` and `Arc<Condvar>` for cross-thread communication.

### Task Lifecycle

All tasks (scheduled and future concurrent) share a unified lifecycle:

| State       | Description                          |
| ----------- | ------------------------------------ |
| `Pending`   | Created but not yet executing        |
| `Running`   | Currently executing                  |
| `Completed` | Finished successfully with a result  |
| `Failed`    | Finished with an error               |
| `Cancelled` | Cancelled before or during execution |

```flux
let t = after 5s { return 42 }
print(task_state(t))    // "pending"
cancel(t)
print(task_state(t))    // "cancelled"
```

### Task State Introspection

| Function          | Returns | Description                             |
| ----------------- | ------- | --------------------------------------- |
| `task_state(t)`   | String  | Current task lifecycle state            |
| `is_done(t)`      | Boolean | true if Completed, Failed, or Cancelled |
| `is_cancelled(t)` | Boolean | true if Cancelled                       |
| `is_running(t)`   | Boolean | true if Running                         |

### Cancellation

Cancellation is cooperative. `cancel(task)` sets the task state to Cancelled. A running task is not forcefully terminated — it completes its current yield-point cycle.

### State Isolation

Concurrent computations should not automatically gain unsafe shared mutable access to the parent's state. The intended boundary:

```
Parent environment
        │
        │ controlled capture (copy/clone)
        ▼
Concurrent computation (isolated)
```

### Scheduler Relationship

```
                 Flux Runtime
                      │
          ┌───────────┴───────────┐
          │                       │
      Scheduler             (Future) Executor
          │                       │
     when to run             how to run
          │                       │
          └───────────┬───────────┘
                      │
                  Task runtime
```

The scheduler handles temporal scheduling. `spawn` creates real OS threads for concurrent execution.

### Concurrent Tasks (`spawn`)

`spawn { body }` creates a new OS thread with an isolated deep-cloned environment:

```flux
let t = spawn { return 42 }
print("parent continues")
// spawned task runs on a real OS thread
```

- Returns a `Task` handle
- Parent execution continues immediately
- Task runs on a real OS thread (parallel execution)
- Task captures a deep clone of the current environment (isolated)
- Result/error stored on the Task handle via `Arc<Mutex<...>>`
- Compatible with `await`, `cancel`, `is_done`, `task_state`

### Task Joining

`join_all(tasks)` waits for all tasks in an array to complete and returns their results:

```flux
let t1 = spawn { return 1 }
let t2 = spawn { return 2 }
let results = join_all([t1, t2])
print(results)    // [1, 2]
```

### Channels

Channels provide FIFO message passing between computations:

```flux
let ch = channel()
send(ch, 42)
print(receive(ch))    // 42
```

| Function            | Description                           |
| ------------------- | ------------------------------------- |
| `channel()`         | Create a new channel                  |
| `send(ch, value)`   | Send a value to the channel           |
| `receive(ch)`       | Receive the next value (nil if empty) |
| `close_channel(ch)` | Close the channel                     |

- Channels use identity equality (same channel = equal)
- Sending on a closed channel produces a runtime error
- Receiving from an empty channel returns `nil`
- FIFO ordering guaranteed
- Any Flux value can be sent through a channel

### Actor Pattern

Actors can be built using channels and spawn:

```flux
let inbox = channel()

spawn {
    let msg = receive(inbox)
    // process message
}

send(inbox, "hello")
```

### Task Introspection

| Function          | Returns       | Description                             |
| ----------------- | ------------- | --------------------------------------- |
| `task_state(t)`   | String        | Current lifecycle state name            |
| `is_done(t)`      | Boolean       | true if Completed, Failed, or Cancelled |
| `is_cancelled(t)` | Boolean       | true if Cancelled                       |
| `is_running(t)`   | Boolean       | true if Running                         |
| `is_failed(t)`    | Boolean       | true if Failed                          |
| `task_result(t)`  | Value or nil  | Task result (nil if not completed)      |
| `task_error(t)`   | String or nil | Task error message (nil if no error)    |

### Channel Introspection

| Function                | Returns | Description                 |
| ----------------------- | ------- | --------------------------- |
| `channel_len(ch)`       | Integer | Number of buffered messages |
| `is_channel_closed(ch)` | Boolean | Whether channel is closed   |

### Channel Semantics

- `receive()` is **non-blocking**: returns `nil` if the channel is empty
- `send()` on a closed channel produces a runtime error
- `receive()` on a closed channel returns buffered messages first, then `nil`
- Any Flux value can be sent through a channel, including Events and Tasks
- FIFO ordering is always preserved

### Execution Model

Flux uses a **hybrid concurrency model**:

- **Scheduled tasks** (`after`, `every`, `at`) execute cooperatively on the main interpreter thread via the scheduler
- **Spawned tasks** (`spawn`) execute on **real OS threads** with isolated interpreter instances

```flux
// This runs on a real OS thread:
let t = spawn { return expensive_work() }

// Parent continues immediately
print("parent working")

// await blocks until the thread completes
let result = await t
```

#### spawn — Real OS Threads

Each `spawn { body }` creates a new OS thread with:

1. A **deep-cloned environment** (no shared Rc references with parent)
2. A fresh interpreter instance on the worker thread
3. A thread-safe `FluxTask` handle (Arc<Mutex></mutex>) for result communication
4. The task body's AST (pure data, trivially Send)

Values captured by a spawned task are **deep-copied** — mutations in the spawned task do not affect the parent, and vice versa. This provides memory safety without data races.

#### Channels — Thread-Safe Communication

Channels use `Arc<Mutex<...>>` internally and are safe for cross-thread communication:

```flux
let ch = channel()
let t = spawn { send(ch, 42) }
await t
print(receive(ch))    // 42
```

#### Task State — Thread-Safe

FluxTask state (Pending/Running/Completed/Failed/Cancelled) uses `Arc<Mutex<...>>` with a `Condvar` for efficient waiting. `await` blocks using the condvar — no busy polling.

#### Await — Proper Synchronization

`await task` uses condvar-based waiting — the calling thread blocks efficiently until the task thread completes, without polling or sleeping.

### Deferred Concurrency Features

- Blocking channel receive
- Select/multiplex across channels
- Actor syntax sugar
- Worker pool with configurable thread count

---

## Type System

Flux has an optional, gradual type system. Type annotations are optional — existing untyped programs continue to work unchanged.

### Type Model

Every Flux runtime value has a corresponding type:

| Type       | Value Examples  | type_of()     |
| ---------- | --------------- | ------------- |
| `Nil`      | `nil`           | `"Nil"`       |
| `Bool`     | `true`, `false` | `"Bool"`      |
| `Int`      | `42`, `-1`      | `"Int"`       |
| `Float`    | `3.14`          | `"Float"`     |
| `String`   | `"hello"`       | `"String"`    |
| `Array`    | `[1, 2, 3]`     | `"Array"`     |
| `Map`      | `{"a": 1}`      | `"Map"`       |
| `Function` | `fn(x) { x }`   | `"() -> Any"` |
| `Range`    | `1..5`          | `"Range"`     |
| `Duration` | `5s`, `100ms`   | `"Duration"`  |
| `Instant`  | `now()`         | `"Instant"`   |
| `Date`     | `date(...)`     | `"Date"`      |
| `Time`     | `time(...)`     | `"Time"`      |
| `DateTime` | `datetime(...)` | `"DateTime"`  |
| `Task`     | `spawn { }`     | `"Task"`      |
| `Channel`  | `channel()`     | `"Channel"`   |
| `Event`    | `event(...)`    | `"Event"`     |
| `Error`    | `error(...)`    | `"Error"`     |
| `Any`      | (any value)     | —             |

### Type Annotations

Type annotations are optional and use `:` syntax:

```flux
let x: Int = 10
let name: String = "Ron"
let active: Bool = true
let nums: Array<Int> = [1, 2, 3]
```

Untyped declarations remain valid:

```flux
let x = 10
```

### Function Return Types

Functions can declare return types using `->`:

```flux
fn add(a, b) -> Int {
    return a + b
}
```

### Generic Types

Collection types support generic parameters:

```flux
Array<Int>
Array<String>
Map<String, Int>
Task<Int>
Channel<String>
Event<Int>
```

### Type Introspection

| Function           | Returns | Description                |
| ------------------ | ------- | -------------------------- |
| `type(v)`          | String  | Runtime type name (legacy) |
| `type_of(v)`       | String  | Flux type descriptor       |
| `is_type(v, name)` | Boolean | Check type compatibility   |

### Type Compatibility

- `Int` and `Float` are mutually compatible (numeric coercion)
- `Any` is compatible with all types
- Unparameterized `Array` matches any `Array<T>`
- Unparameterized `Map` matches any `Map<K, V>`
- `Nil` is compatible with optional types

### Runtime Type Checking

Type annotations are enforced at runtime. Assigning a value that doesn't match produces a type error:

```flux
let x: Int = "hello"       // type error: expected Int, found String
let d: Duration = 42       // type error: expected Duration, found Int
```

### Typed Function Parameters

Function parameters support type annotations:

```flux
fn add(a: Int, b: Int) -> Int {
    return a + b
}

fn greet(name: String) {
    print("Hello, " + name)
}
```

Type mismatches at call sites produce runtime errors:

```flux
greet(42)    // type error: expected String, found Int
```

Mixed typed/untyped parameters are supported:

```flux
fn f(a: Int, b) { return a + b }
```

### Typed Closures

Anonymous functions support typed parameters:

```flux
let f = fn(x: Int) { return x * 2 }
f(5)        // 10
f("bad")    // type error
```

### Optional Types (Internal)

The type system represents optional types as `T?`:

- `Int?` is compatible with `Int` and `Nil`
- Optional types can be used in compatibility checks

### Union Types (Internal)

The type system represents union types as `T | U`:

- `Int | String` is compatible with both `Int` and `String`
- Union types can be used in compatibility checks

### Type Narrowing

Use `is_type()` for runtime type checking and narrowing:

```flux
if is_type(x, "Int") {
    // x is known to be Int here
}
```

### Type Diagnostics

Type errors include expected and actual type information:

```
type error: expected String, found Int
type error: expected Bool, found Int
```

### Generic Functions

Functions can declare generic type parameters:

```flux
fn identity<T>(x: T) -> T {
    return x
}

print(identity(42))       // 42
print(identity("hello"))  // hello
```

Multiple type parameters are supported:

```flux
fn pair<A, B>(a: A, b: B) { ... }
```

Generic parameters are stored in the AST and resolved at runtime from the argument types.

### Type Aliases

Type aliases create alternative names for existing types:

```flux
type UserId = Int
type Name = String
```

Aliases resolve transparently — they do not create distinct runtime types.

### User-Defined Structured Types

Structured types define named records with typed fields:

```flux
type User {
    id: Int,
    name: String
}
```

Instances are created with `make_struct()`:

```flux
let user = make_struct("User", {"id": 42, "name": "Ron"})
print(user.id)      // 42
print(user.name)    // Ron
print(type(user))   // User
```

Struct values support:

- Named field access via `.field`
- Display formatting: `User { id: 42, name: "Ron" }`
- Type introspection via `type()`
- Equality (identity-based)

### Deferred Type Features

- Static type checking at parse time (ahead-of-execution analysis)
- Generic type constraints
- Nominal type identity for aliases
- Method definitions on types
- Typed destructuring annotations (`let [a, b]: Array<Int> = [1, 2]`)

---

## Packages & Ecosystem

### Project Structure

A Flux project uses a standard layout:

```
flux.toml
src/
    main.flux
    ...
```

The project root is identified by the presence of `flux.toml`. The runtime searches upward from the current directory.

### Project Manifest (`flux.toml`)

```toml
[package]
name = "myproject"
version = "0.1.0"

[dependencies]
utils = { path = "../utils" }
```

Required fields:

- `name` — package name (string)
- `version` — semantic version (`MAJOR.MINOR.PATCH`)

### Dependencies

Dependencies are declared in the `[dependencies]` section. Currently supported:

- **Path dependencies**: `name = { path = "../path/to/package" }`

Each dependency must have its own `flux.toml` manifest.

### Version Handling

Versions follow semantic versioning (`MAJOR.MINOR.PATCH`). Supported version requirements:

- Exact: `"1.2.3"`
- Caret: `"^1.2.0"` (compatible with same major version)
- Greater-or-equal: `">=1.0.0"`

### Dependency Resolution

Dependencies are resolved transitively. Circular dependencies are detected and produce a clear diagnostic:

```
circular dependency detected: A -> B -> C -> A
```

### Package Imports

Package imports use the existing module system:

```flux
import utils
import utils.math
from utils import helper
```

### CLI Commands

| Command            | Description                              |
| ------------------ | ---------------------------------------- |
| `flux init [name]` | Initialize a new Flux project            |
| `flux deps`        | Show project dependencies and resolution |
| `flux <file>`      | Run a Flux source file                   |
| `flux run <file>`  | Run a Flux source file                   |

### Module Caching

Modules are cached per execution. Re-importing the same module returns the same state.

### Standalone Files

A standalone `.flux` file can be run without a `flux.toml` manifest. The package system is optional.

### Deferred Package Features

- Remote package registry
- Package publishing
- Git/HTTP dependencies
- Cryptographic package signing
- Lock file generation
- Automatic dependency updates

---

## Developer Tooling

### CLI Commands

| Command                   | Description                        |
| ------------------------- | ---------------------------------- |
| `flux`                    | Start interactive REPL             |
| `flux <file>`             | Run a Flux program                 |
| `flux run <file>`         | Run a Flux program                 |
| `flux check [file]`       | Check for errors without executing |
| `flux fmt [file]`         | Format Flux source code            |
| `flux fmt [file] --check` | Check formatting without modifying |
| `flux test [dir]`         | Run Flux test files                |
| `flux lint [file]`        | Check for common issues            |
| `flux init [name]`        | Initialize a new Flux project      |
| `flux deps`               | Show project dependencies          |
| `flux repl`               | Start interactive REPL             |
| `flux --version`          | Print version                      |
| `flux --help`             | Print help                         |

### Developer Workflow

```
flux init myapp
cd myapp
flux check
flux fmt
flux test
flux run
```

### `flux check`

Validates Flux source without executing. Detects lexical and parse errors. Returns exit code 0 for valid, non-zero for errors.

### `flux fmt`

Formats Flux source code into canonical form using the AST. Running `flux fmt` twice produces identical output (idempotent). Use `--check` to verify formatting without modifying files.

### `flux test`

Discovers and runs `.flux` files in the `tests/` directory. Reports passed/failed/total counts. Returns non-zero exit code if any test fails.

### `flux lint`

Performs lightweight static analysis. Detects:

- Empty blocks (if, while, for, function bodies)
- Other structural issues

### REPL Commands

| Command             | Description                    |
| ------------------- | ------------------------------ |
| `:help`             | Show available commands        |
| `:clear` / `:reset` | Reset the environment          |
| `:version`          | Show Flux version              |
| `:type <expr>`      | Show the type of an expression |
| `:load <file>`      | Load and execute a Flux file   |
| `:quit` / `:exit`   | Exit the REPL                  |

### Diagnostics

Errors include source location, message, and source context:

```
filename:line:column: error: message

source line
         ^^^^ indicator
```

Runtime errors include stack traces when available:

```
error: division by zero

stack:
  calculate() at math.flux:18
  process()   at app.flux:42
```

### Deferred Tooling Features

- Language Server Protocol (LSP)
- Full debugger with breakpoints
- Advanced formatter with configuration
- Code completion
- Go-to-definition
- Find references
- Rename refactoring
- Advanced linting rules
- JSON diagnostic output
