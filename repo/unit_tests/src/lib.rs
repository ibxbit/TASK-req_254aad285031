//! `unit_tests` — Rust unit-test crate.
//!
//! Exercises the public surface of `shared` (DTOs, enums) that cross the
//! HTTP boundary. Internal backend units (argon2, DCT pHash, deadline math,
//! hash chain, etc.) are covered by `#[cfg(test)]` modules inside their
//! respective source files — `cargo test --workspace` runs both sets.
//!
//! Tests live in `tests/*.rs` (one file per concern).

// No production code — just lets Cargo treat this as a library crate so the
// `tests/` directory is compiled as integration-style unit tests.
