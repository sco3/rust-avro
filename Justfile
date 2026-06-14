# Justfile for rust-avro project
# This file defines common development tasks using the just task runner.
# It provides shortcuts for building, testing, formatting, linting, and more.

# Run the default task (list available targets) when just is invoked without arguments.
default:
    @just --color never --list

# Format the code using rustfmt.
fmt:
    cargo fmt

# Lint the code using clippy with pedantic lints and treat warnings as errors.
clippy:
    cargo clippy -- -D warnings -W clippy::pedantic

# Run the test suite.
test:
    cargo test

# Run all tests, including those with all features enabled.
test-all:
    cargo test --all-features

# Perform a cargo check (type checking only).
check:
    cargo check

# Build the project in debug mode.
build:
    cargo build

# Build the project in release mode.
release:
    cargo build --release

# Run the main binary of the project.
run:
    cargo run

# Print the list of available recipes.
# This target is used as the default so that invoking just without
# arguments shows the help/available targets instead of running the test suite.
list:
    @just --list

# Run the `convert_schema` binary with additional arguments.
run-convert-schema:
    cargo run --bin convert_schema -- $*
# Run the `converts` binary with the specific arguments required for the
# tdr6021 struct conversion.
convert-6021:
    cargo run --bin converts -- -i data/tdr6021.struct.line -o data/tdr6021.struct.avsc -n tdr6021

# Generate documentation and open it in the browser.
doc:
    cargo doc --open

# Clean build artifacts.
clean:
    cargo clean

# Run benchmarks (if any are defined).
bench:
    cargo bench
