# Neuro Language Specification (Phase 2)

## 1. Introduction
Neuro is a statically typed, domain-specific language designed with a "security-first" and zero-trust philosophy. It compiles down to highly optimized LLVM IR while mathematically guaranteeing memory safety and logical consistency at the language level.

## 2. Core Syntax & Keywords

### Keywords
- `fn`: Defines a function.
- `let`: Defines an immutable variable.
- `mut`: Keyword modifier to define a mutable variable.
- `type`: Defines custom types or structs (future phase).
- `if` / `else`: Control flow branches.
- `return`: Return from a function.

### Variable Declarations
Variables are strongly typed and must be initialized upon declaration to enforce memory safety.
```neuro
// Immutable declaration
let x: int = 5;

// Mutable declaration
let mut y: float = 3.14;
```

### Functions
Functions use `fn` and require explicit return types.
```neuro
fn calculate(a: int, b: int) -> int {
    return a * b;
}

fn main() -> int {
    let mut counter: int = calculate(2, 5);
    return 0;
}
```

## 3. Zero-Trust Security Rules
The Analyzer strictly enforces the following constraints:
1. **No Uninitialized Variables**: A variable must have an initial value. Default garbage values are impossible to read.
2. **Immutability by Default**: All variables are immutable unless explicitly marked with `mut`.
3. **Strict Type Coercion**: Implicit conversions (e.g., `float` to `int`) are forbidden. Explicit casting must be used.
4. **No Unsafe Pointers**: Direct memory addresses and pointer arithmetic are disallowed at the user level. Array indexing is bounds-checked at runtime by the C Foundation runtime.
