//! Repository layer — pure data access.
//!
//! Contracts:
//! * Takes `&MySqlPool` (or a transaction) and primitive/DTO arguments.
//! * Returns `sqlx::Result<T>` with domain-shaped types.
//! * No business rules, no role checks, no HTTP types, no audit writes.
//! * One module per domain aggregate.

pub mod users;
