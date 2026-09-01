default:
    @echo "Available targets: bench-dev, bench-release, bench-cranelift, build-dev, build-release, check"

build-dev:
    cargo build --bin electronpy

build-release:
    cargo build --release --bin electronpy

check:
    cargo check --bin electronpy

bench-dev:
    python benchmarks/run_benchmarks.py --preset dev

bench-release:
    python benchmarks/run_benchmarks.py --preset release

bench-cranelift:
    python benchmarks/run_benchmarks.py --preset cranelift
