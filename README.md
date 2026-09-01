<<<<<<< HEAD
# pyvolt
=======
# ElectronPy

Version: 0.0.1

A Rust-based compiler/transpiler for a statically analyzable subset of Python.

## Product goal

ElectronPy is designed for compute-heavy Python scripts that fit a clear subset of semantics and can be translated to native Rust safely and predictably.

This is intentionally not a full Python interpreter replacement. The compiler is focused on reliable, benchmarkable workloads rather than universal compatibility.

## Supported subset

The current supported subset includes:
- integer, float, bool, string, and None literals
- variable assignment and reassignment
- arithmetic: +, -, *, /
- comparisons: ==, !=, <, <=, >, >=
- `print(...)`
- `if / else`
- `while` loops
- `for ... in range(...)` loops
- simple user-defined functions
- function parameter and return type annotations such as `a: int`, `-> int`

## Compiler pipeline

Python source -> CPython AST JSON -> ElectronPy AST -> IR -> optimization -> generated Rust source -> native binary

## Quick start

Build the CLI:

```bash
cargo build --release --bin electronpy
```

Source-only mode (no Rust install required):

```bash
electronpy compile examples/simple.py output.rs
# or explicitly:
electronpy compile examples/simple.py output.rs --source-only
```

Direct native EXE flow (requires Rust for final build):

```bash
electronpy build examples/simple.py app.exe
```

Or the shorthand:

```bash
electronpy examples/simple.py output.rs
```

Analyze a file:

```bash
electronpy analyze examples/simple.py
```

Run a benchmark:

```bash
electronpy benchmark examples/simple.py examples/simple.rs
```

## Source-only vs EXE mode

ElectronPy is intentionally split into two execution stages:

- Source mode: `electronpy compile ...` emits Rust source only. This path does not require a Rust toolchain and is the safest way to use ElectronPy in constrained environments.
- EXE mode: `electronpy build ...` or `electronpy run ...` performs the full transpile + Rust compile + native output flow. This requires a working Rust toolchain.

This clean separation preserves a reliable source-generation workflow while retaining a high-performance native build path for developers who want direct executables.

## Example

```python
def add(a: int, b: int) -> int:
    return a + b

print(add(5, 7))
```

This compiles to Rust and runs as a native binary.

## 0.0.1 release notes

ElectronPy 0.0.1 is a release candidate for a narrow, benchmarkable Python-to-Rust compiler subset. It is designed for deterministic compute kernels and loop-heavy numeric code where a static subset can be lowered to native Rust with predictable behavior.

### What this release includes

- native AST-driven Python-to-Rust transpilation for a supported subset
- strict output validation against CPython for the audited workloads
- honest fresh-code generation and compile timing on each run
- safe CLI and benchmark execution with path validation and explicit environment hygiene
- benchmark scaffolding for CPython, ElectronPy, PyPy, Numba, Nuitka, Codon, and Rust when those tools are installed

### Install

```bash
# from the project root
cargo build --release --bin electronpy
# or use the generated binary directly
./target/release/electronpy.exe --help
```

On Windows PowerShell:

```powershell
cargo build --release --bin electronpy
.\target\x86_64-pc-windows-msvc\release\electronpy.exe --help
```

### Exact benchmark commands for 0.0.1 users

```bash
# compile a single Python file to Rust
./target/release/electronpy.exe compile examples/simple.py output.rs

# benchmark against local workload files
./target/release/electronpy.exe benchmark examples/simple.py examples/simple.rs --repeats 10

# run the full runtime matrix across CPython, PyPy, Numba, Nuitka, Codon, ElectronPy, and Rust
python benchmarks/benchmark_matrix.py --repeats 3 --runtime all

# compare generated code against the fair dynamic reference workload
bash scripts/verify_same_output.sh
```

### Example workload used for validation

```python
# same mathematical workload used for fair-output validation
sum_total = 0
for i in range(10_000_001):
    sum_total += i
print(sum_total)
```

Expected output:

```text
50000005000000
```

## Production positioning

ElectronPy is best thought of as a useful compiler for a narrow but realistic subset of Python, especially for:
- numerical workloads
- loop-heavy data processing
- static compute kernels
- performance-critical script translation

The current philosophy is: reliable subset, honest benchmarking, clear boundaries, and better local iteration speed.

## Product overview

For a polished product-level overview aimed at real-world adoption, see [docs/product-overview.md](docs/product-overview.md).

For practical use cases and enterprise adoption patterns, see [docs/use-cases.md](docs/use-cases.md).

For the 1.0 milestone plan and production roadmap, see [docs/roadmap-v1.md](docs/roadmap-v1.md).

For the exact 1.0 supported subset and explicit unsupported-case policy, see [docs/subset-contract.md](docs/subset-contract.md).

These documents cover:
- why ElectronPy matters for Python-heavy compute workloads
- who should use it and when
- source-only vs native executable deployment modes
- real enterprise-grade use cases and adoption patterns
- honest positioning for a subset compiler in production environments
- the next major-version plan for a stable 1.0 release
- the strict language contract that defines the official 1.0 support boundary
>>>>>>> master
